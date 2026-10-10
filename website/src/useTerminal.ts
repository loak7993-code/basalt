import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Drives a looping terminal transcript: commands type character by character,
 * output lines appear in sequence, and the install shows a live progress bar
 * with evolving counters — matching the real CLI's output format.
 */

type Step =
  | { kind: "type"; text: string; speed?: number }
  | { kind: "out"; text: string; tone?: "dim" | "accent" | "plain" }
  | { kind: "prog"; from: number; to: number; duration: number; total: number; mbTotal: number }
  | { kind: "wait"; ms: number };

const SCRIPT: Step[] = [
  { kind: "type", text: "basalt install 26.3" },
  { kind: "out", text: "downloading 5199 files ...", tone: "dim" },
  { kind: "prog", from: 0, to: 1, duration: 2600, total: 5199, mbTotal: 559.9 },
  { kind: "out", text: "installed: ~/.local/share/basalt/versions/26.3", tone: "accent" },
  { kind: "wait", ms: 700 },
  { kind: "type", text: "basalt account steve" },
  { kind: "out", text: "account stored: steve (a1165a91-6696-3381-d61e-756be7cb3fad)", tone: "accent" },
  { kind: "wait", ms: 700 },
  { kind: "type", text: "basalt play 26.3" },
  { kind: "out", text: "java: /usr/bin/java (major 25)", tone: "dim" },
  { kind: "out", text: "launching 26.3 with steve (1637M heap, 25 args)", tone: "accent" },
  { kind: "wait", ms: 4200 },
];

export type TerminalLine = { text: string; tone?: "dim" | "accent" | "plain" };

export type ProgressFrame = {
  fraction: number;
  done: number;
  total: number;
  mb: number;
};

const BLOCKS = 22;

/** Filled and empty block counts for the progress bar. */
function barSegments(fraction: number): { filled: number; empty: number } {
  const filled = Math.max(0, Math.min(BLOCKS, Math.round(fraction * BLOCKS)));
  return { filled, empty: BLOCKS - filled };
}

export function useTerminalTranscript() {
  const [stepIndex, setStepIndex] = useState(0);
  const [typed, setTyped] = useState(0);
  const [progress, setProgress] = useState<ProgressFrame | null>(null);
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const reducedMotion = useRef(false);

  useEffect(() => {
    reducedMotion.current =
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  }, []);

  // When the step index changes, reset per-step state.
  useEffect(() => {
    setTyped(0);
    setProgress(null);
  }, [stepIndex]);

  const advance = useCallback(() => {
    setStepIndex((i) => (i + 1) % SCRIPT.length);
    if (stepIndex === SCRIPT.length - 1) {
      // Looping back to the top: clear the screen like a fresh shell.
      setLines([]);
    }
  }, [stepIndex]);

  useEffect(() => {
    const step = SCRIPT[stepIndex];
    if (!step) return;

    if (step.kind === "type") {
      if (reducedMotion.current) {
        setLines((prev) => [...prev, { text: `❯ ${step.text}`, tone: "plain" }]);
        advance();
        return;
      }
      let cancelled = false;
      let i = 0;
      const typeNext = () => {
        if (cancelled) return;
        i++;
        setTyped(i);
        if (i < step.text.length) {
          setTimeout(typeNext, 34 + Math.random() * 42);
        } else {
          setLines((prev) => [...prev, { text: `❯ ${step.text}`, tone: "plain" }]);
          setTimeout(() => !cancelled && advance(), 260);
        }
      };
      const first = setTimeout(typeNext, 160);
      return () => {
        cancelled = true;
        clearTimeout(first);
      };
    }

    if (step.kind === "out") {
      const t = setTimeout(() => {
        setLines((prev) => [...prev, { text: step.text, tone: step.tone ?? "dim" }]);
        advance();
      }, reducedMotion.current ? 0 : 260);
      return () => clearTimeout(t);
    }

    if (step.kind === "prog") {
      if (reducedMotion.current) {
        setProgress({ fraction: 1, done: step.total, total: step.total, mb: step.mbTotal });
        advance();
        return;
      }
      let raf = 0;
      const start = performance.now();
      const tick = (now: number) => {
        const t = Math.min((now - start) / step.duration, 1);
        const eased = 1 - Math.pow(1 - t, 2);
        const fraction = step.from + (step.to - step.from) * eased;
        setProgress({
          fraction,
          done: Math.round(fraction * step.total),
          total: step.total,
          mb: fraction * step.mbTotal,
        });
        if (t < 1) raf = requestAnimationFrame(tick);
        else setTimeout(() => advance(), 350);
      };
      raf = requestAnimationFrame(tick);
      return () => cancelAnimationFrame(raf);
    }

    if (step.kind === "wait") {
      const t = setTimeout(advance, reducedMotion.current ? 0 : step.ms);
      return () => clearTimeout(t);
    }
  }, [stepIndex, advance]);

  const step = SCRIPT[stepIndex];
  const typing = step?.kind === "type";
  const typedText = typing ? step.text.slice(0, typed) : "";
  const typingDone = typing ? typed >= step.text.length : true;

  return {
    lines,
    // While typing, the in-progress command renders with the block cursor.
    typingLine: typing && !typingDone ? `❯ ${typedText}` : null,
    progress: progress
      ? (() => {
          const { filled, empty } = barSegments(progress.fraction);
          return {
            filled: "█".repeat(filled),
            tail: "░".repeat(empty),
            label: `${progress.done.toLocaleString()}/${progress.total.toLocaleString()} files · ${progress.mb.toFixed(1)} MB`,
          };
        })()
      : null,
  };
}
