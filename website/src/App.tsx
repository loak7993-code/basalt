import { useCallback, useEffect, useRef, useState } from "react";
import { motion, useInView } from "motion/react";
import {
  IconCheck,
  IconCopy,
  IconCpu,
  IconDownload,
  IconGithub,
  IconGlobe,
  IconLightning,
  IconTerminal,
  IconUsers,
  IconX,
} from "./components/icons";

const EASE = [0.16, 1, 0.3, 1] as const;

const REPO = "https://github.com/loak7993-code/basalt";
const RELEASES = `${REPO}/releases`;
const APPIMAGE_SHA256 =
  "3f09bb47d074aa90f583d51effa8bddb01e62dc58b8bcd2b581ee295e13ec0c2";
const LINUX_RUN =
  "curl -LO https://github.com/loak7993-code/basalt/releases/download/v2.0.0/Basalt-2.0.0-x86_64.AppImage && chmod +x Basalt-2.0.0-x86_64.AppImage && ./Basalt-2.0.0-x86_64.AppImage";

function Mark({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 108 108" aria-hidden="true">
      <rect width="108" height="108" rx="22" fill="#0A0A0C" />
      <path d="M36,58 46,54 46,72 36,72 Z M34,74.5 74,74.5 74,76.5 34,76.5 Z" fill="#2E6FC7" />
      <path d="M48,44 58,40 58,72 48,72 Z" fill="#4D9EFF" />
      <path d="M60,56 70,52 70,72 60,72 Z" fill="#2E6FC7" />
    </svg>
  );
}

function Reveal({
  children,
  delay = 0,
  className,
}: {
  children: React.ReactNode;
  delay?: number;
  className?: string;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 28 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-60px" }}
      transition={{ duration: 0.55, delay, ease: EASE }}
      className={className}
    >
      {children}
    </motion.div>
  );
}

/* Animated counter */
function Counter({ target, suffix = "", duration = 1200 }: { target: number; suffix?: string; duration?: number }) {
  const ref = useRef<HTMLSpanElement>(null);
  const inView = useInView(ref, { once: true });
  const [value, setValue] = useState(0);

  useEffect(() => {
    if (!inView) return;
    const start = performance.now();
    let raf: number;
    const tick = (now: number) => {
      const t = Math.min((now - start) / duration, 1);
      const eased = 1 - Math.pow(1 - t, 3);
      setValue(Math.round(eased * target));
      if (t < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [inView, target, duration]);

  return (
    <span ref={ref}>
      {value.toLocaleString()}
      {suffix}
    </span>
  );
}

const NAV = [
  { label: "Features", href: "#features" },
  { label: "Benchmarks", href: "#bench" },
  { label: "Screens", href: "#screens" },
  { label: "Download", href: "#download" },
];

type Shot = { src: string; alt: string; label: string };

const SHOTS: Shot[] = [
  { src: "/shots/home-263.png", alt: "Home screen with 26.3 installed and an offline account", label: "Home" },
  { src: "/shots/download-263.png", alt: "Download list with 26.3 at the top of releases", label: "Downloads" },
  { src: "/shots/offline-account.png", alt: "Offline account created and selected", label: "Accounts" },
  { src: "/shots/home-empty.png", alt: "Empty home state pointing at the download action", label: "First run" },
];

const CHECKS = [
  "Version install from the official Mojang manifest",
  "Offline accounts, Microsoft accounts, authlib-injector",
  "Forge, Fabric, NeoForge and OptiFine installers",
  "Control layout editor, gamepad, gyroscope support",
  "Renderer selection: gl4es, ANGLE, Zink, VirGL",
  "Terracotta multiplayer rooms",
];

/* Install timings measured on the verification machine, fresh 26.3 installs
   run back to back. Both numbers per row come from the same session. */
const BENCH_MAX = 120;
const BENCH = [
  {
    title: "Android, x86_64 emulator",
    rows: [
      { label: "Kotlin download engine", seconds: 120, accent: false },
      { label: "Rust download engine", seconds: 37, accent: true },
    ],
    note: "5,199 files, SHA1 verified, same emulator, same manifest.",
  },
  {
    title: "Linux, same machine",
    rows: [
      { label: "Previous installer", seconds: 30, accent: false },
      { label: "Rust installer", seconds: 13, accent: true },
    ],
    note: "The previous installer also hung past 20 minutes on one run. The Rust one holds a 60s per-file timeout and three attempts.",
  },
];

/* One command, copyable. Clipboard access can be refused (insecure context,
   permission denied), so the failure path says so instead of silently doing
   nothing. */
function CopyChip({ command }: { command: string }) {
  const [state, setState] = useState<"idle" | "copied" | "blocked">("idle");

  useEffect(() => {
    if (state === "idle") return;
    const t = window.setTimeout(() => setState("idle"), 2200);
    return () => window.clearTimeout(t);
  }, [state]);

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(command);
      setState("copied");
    } catch {
      setState("blocked");
    }
  }, [command]);

  return (
    <div className="mt-6 flex max-w-xl items-center gap-3 rounded-xl border border-white/[0.07] bg-black/40 px-4 py-3 backdrop-blur-sm">
      <code className="min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-400">
        {command}
      </code>
      <button
        type="button"
        onClick={copy}
        aria-label="Copy the Linux install command"
        className="inline-flex shrink-0 items-center gap-1.5 rounded-md border border-white/[0.1] px-2.5 py-1.5 text-[12px] text-zinc-300 transition-colors hover:border-white/25 hover:text-white active:scale-[0.98]"
      >
        {state === "copied" ? (
          <>
            <IconCheck size={14} className="text-accent" />
            Copied
          </>
        ) : state === "blocked" ? (
          "Select it and copy"
        ) : (
          <>
            <IconCopy size={14} />
            Copy
          </>
        )}
      </button>
    </div>
  );
}

/* Install timing bars. Width is a static value, the animation is scaleX so
   nothing triggers layout. */
function BenchBar({
  label,
  seconds,
  accent,
}: {
  label: string;
  seconds: number;
  accent: boolean;
}) {
  return (
    <div>
      <div className="flex items-baseline justify-between gap-3">
        <span className="text-[13px] text-zinc-400">{label}</span>
        <span className="font-mono text-[12px] text-zinc-300">{seconds}s</span>
      </div>
      <div className="mt-2 h-2.5 w-full overflow-hidden rounded-full bg-white/[0.04]">
        <motion.div
          className={`h-full rounded-full ${accent ? "bg-accent" : "bg-zinc-600"}`}
          style={{
            width: `${(seconds / BENCH_MAX) * 100}%`,
            transformOrigin: "left center",
          }}
          initial={{ scaleX: 0 }}
          whileInView={{ scaleX: 1 }}
          viewport={{ once: true, margin: "-40px" }}
          transition={{ duration: 0.75, ease: EASE }}
        />
      </div>
    </div>
  );
}

function Lightbox({ shot, onClose }: { shot: Shot; onClose: () => void }) {
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    document.body.style.overflow = "hidden";
    closeRef.current?.focus();
    return () => {
      window.removeEventListener("keydown", onKey);
      document.body.style.overflow = "";
    };
  }, [onClose]);

  return (
    <motion.div
      role="dialog"
      aria-modal="true"
      aria-label={`${shot.label} screenshot`}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/85 p-4 backdrop-blur-sm"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.2 }}
      onClick={onClose}
    >
      <motion.figure
        className="w-full max-w-5xl overflow-hidden rounded-xl border border-white/10 bg-panel"
        initial={{ scale: 0.97, y: 12 }}
        animate={{ scale: 1, y: 0 }}
        transition={{ duration: 0.3, ease: EASE }}
        onClick={(e) => e.stopPropagation()}
      >
        <img src={shot.src} alt={shot.alt} className="w-full" />
        <figcaption className="flex items-center justify-between border-t border-white/[0.04] px-5 py-3.5">
          <span className="text-sm text-zinc-200">{shot.label}</span>
          <span className="font-mono text-[10px] text-zinc-600">
            Android 15 · 2400×1080
          </span>
        </figcaption>
      </motion.figure>
      <button
        ref={closeRef}
        type="button"
        onClick={onClose}
        aria-label="Close screenshot"
        className="absolute right-5 top-5 inline-flex h-11 w-11 items-center justify-center rounded-lg border border-white/10 bg-black/60 text-zinc-300 transition-colors hover:text-white"
      >
        <IconX size={20} />
      </button>
    </motion.div>
  );
}

function App() {
  const [openShot, setOpenShot] = useState<Shot | null>(null);

  return (
    <div className="relative min-h-screen bg-canvas text-zinc-100">
      <a
        href="#download"
        className="skip-link absolute left-4 top-3 z-50 -translate-y-20 rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-black transition-transform focus:translate-y-0"
      >
        Skip to downloads
      </a>
      <div className="glow-haze" />

      {/* Nav */}
      <header className="fixed top-0 left-0 right-0 z-40 border-b border-white/[0.04] backdrop-blur-xl bg-[#050505]/70">
        <div className="mx-auto flex h-14 max-w-5xl items-center justify-between px-5">
          <a href="#top" className="flex items-center gap-2.5">
            <span className="mark-breathe inline-flex"><Mark size={24} /></span>
            <span className="text-sm font-semibold tracking-tight">Basalt</span>
          </a>
          <nav className="flex items-center gap-1 text-[13px] text-zinc-500">
            {NAV.map((item) => (
              <a
                key={item.href}
                href={item.href}
                className="hidden sm:inline-block rounded-lg px-3 py-1.5 transition-colors hover:text-zinc-100"
              >
                {item.label}
              </a>
            ))}
            <a
              href={REPO}
              target="_blank"
              rel="noreferrer"
              className="ml-2 inline-flex items-center gap-1.5 rounded-lg border border-white/[0.08] px-3 py-1.5 text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
            >
              <IconGithub size={14} />
              Source
            </a>
          </nav>
        </div>
      </header>

      <main id="top" className="relative z-10">
        {/* ─── Hero ─── */}
        <section
          className="hero-with-spot relative isolate overflow-hidden"
          onPointerMove={(e) => {
            const target = e.currentTarget;
            const rect = target.getBoundingClientRect();
            target.style.setProperty("--spot-x", `${e.clientX - rect.left}px`);
            target.style.setProperty("--spot-y", `${e.clientY - rect.top}px`);
          }}
        >
          {/* A crop of the real app screen, behind the text. Masked so the headline
                always sits on clean canvas. */}
          <div className="absolute inset-0 z-0 pointer-events-none overflow-hidden" aria-hidden="true">
            <img
              src="/shots/hero-backdrop.png"
              alt=""
              className="hero-shot h-full w-full object-cover object-[64%_center]"
            />
            <div className="absolute inset-0 bg-gradient-to-b from-canvas via-canvas/20 to-canvas" />
          </div>
          <div className="spotlight" aria-hidden="true" />

          <div className="relative z-10">
          <div className="mx-auto flex min-h-[500px] max-w-5xl items-center px-5 pt-28 pb-12 md:min-h-[540px] md:pt-32">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.55, ease: EASE }}
              className="min-w-0 max-w-xl md:max-w-2xl"
            >
              <p className="font-mono text-[11px] tracking-[0.2em] text-accent/80 uppercase">
                GPL-3 · Rust engine · Offline first
              </p>
              <h1 className="mt-5 text-[2.15rem] font-semibold leading-[1.06] tracking-[-0.03em] md:text-[3.8rem] md:leading-[1.02]">
                Minecraft Java Edition,
                <br className="hidden md:inline" />{" "}
                <span className="text-zinc-400">launched by Rust.</span>
              </h1>
              <p className="mt-6 max-w-md text-[15px] leading-relaxed text-zinc-300/90 md:text-base">
                Basalt installs the game from Mojang's own servers and launches
                it on Android and Linux. No account wall for offline play.
              </p>
              <div className="mt-8 flex flex-wrap items-center gap-3">
                <a
                  href="#download"
                  className="inline-flex items-center gap-2 rounded-lg bg-accent px-5 py-2.5 text-sm font-semibold text-black transition-all active:scale-[0.98] hover:brightness-110"
                >
                  <IconDownload size={16} />
                  Download
                </a>
                <a
                  href="#features"
                  className="inline-flex items-center gap-2 rounded-lg border border-white/[0.1] bg-black/30 px-5 py-2.5 text-sm font-medium text-zinc-200 backdrop-blur-sm transition-colors hover:border-white/25 hover:text-white"
                >
                  How it works
                </a>
              </div>
              <CopyChip command={LINUX_RUN} />
            </motion.div>
          </div>

          {/* Stats bar */}
          <Reveal delay={0.2} className="mt-10 md:mt-16">
            <div className="grid grid-cols-2 gap-px rounded-xl bg-white/[0.04] md:grid-cols-4 overflow-hidden">
              {[
                { value: 26.3, label: "Current MC release", mono: true, raw: "26.3" },
                { value: 5199, label: "Files per install", mono: true, suffix: "" },
                { value: 37, label: "Second install (Rust)", mono: true, suffix: "s" },
                { value: 25, label: "Java runtime bundled", mono: true, raw: "25" },
              ].map((stat) => (
                <div key={stat.label} className="bg-panel px-5 py-4">
                  <div className="font-mono text-lg font-medium text-accent/90">
                    {stat.raw ? stat.raw : <Counter target={stat.value} suffix={stat.suffix} />}
                  </div>
                  <div className="mt-0.5 text-xs text-zinc-500">{stat.label}</div>
                </div>
              ))}
            </div>
          </Reveal>
          </div>
        </section>

        {/* ─── Tech marquee ─── */}
        <section className="relative py-6 border-y border-white/[0.04] bg-black/20">
          <div className="marquee-mask overflow-hidden">
            <div className="marquee-track gap-10 px-5">
              {[0, 1].map((copy) => (
                <div key={copy} className="flex shrink-0 items-center gap-10" aria-hidden={copy === 1}>
                  {[
                    "MC 26.3",
                    "Java 25",
                    "gl4es",
                    "ANGLE",
                    "Zink",
                    "VirGL",
                    "Fabric",
                    "Forge",
                    "NeoForge",
                    "OptiFine",
                    "Terracotta",
                    "authlib-injector",
                    "SHA1 verified",
                    "GPL-3",
                  ].map((chip) => (
                    <span
                      key={chip}
                      className="flex items-center gap-3 font-mono text-[11px] tracking-wide text-zinc-600"
                    >
                      <span className="w-1 h-1 rounded-full bg-accent/40" />
                      {chip}
                    </span>
                  ))}
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* ─── Features: bento ─── */}
        <section id="features" className="mx-auto max-w-5xl px-5 py-16 md:py-28">
          <Reveal>
            <h2 className="text-2xl font-semibold tracking-[-0.02em] md:text-[1.75rem]">
              Built different, on purpose
            </h2>
            <p className="mt-3 max-w-xl text-sm text-zinc-400">
              A fork of ZalithLauncher 2 with the launch engine rewritten in
              Rust and the whole product layer rebuilt to one design.
            </p>
          </Reveal>

          {/* Row 1: two wide cards */}
          <div className="mt-10 grid gap-4 md:grid-cols-2">
            <Reveal className="h-full">
              <div className="lift group h-full rounded-xl bg-panel p-7 hover:bg-panel-2">
                <div className="flex items-start justify-between">
                  <IconTerminal size={22} className="text-accent/60" />
                  <span className="font-mono text-[10px] text-zinc-600">engine-rs</span>
                </div>
                <h3 className="mt-5 text-[15px] font-medium">Rust launch engine</h3>
                <p className="mt-2 text-sm leading-relaxed text-zinc-400">
                  The same engine crate drives Android and Linux. It parses the
                  official manifest, filters libraries by platform, applies
                  memory and GC policy, and assembles every JVM and game
                  argument. 18 tests run against the real 26.3 manifest.
                </p>
                <div className="mt-4 flex flex-wrap gap-2 font-mono text-[10px] text-zinc-600">
                  <span className="rounded bg-white/[0.04] px-2 py-0.5">serde</span>
                  <span className="rounded bg-white/[0.04] px-2 py-0.5">SHA1</span>
                  <span className="rounded bg-white/[0.04] px-2 py-0.5">JNI</span>
                  <span className="rounded bg-white/[0.04] px-2 py-0.5">parity-tested</span>
                </div>
              </div>
            </Reveal>
            <Reveal delay={0.06} className="h-full">
              <div className="lift group h-full rounded-xl bg-panel p-7 hover:bg-panel-2">
                <div className="flex items-start justify-between">
                  <IconUsers size={22} className="text-accent/60" />
                  <span className="font-mono text-[10px] text-zinc-600">no account wall</span>
                </div>
                <h3 className="mt-5 text-[15px] font-medium">Offline accounts first</h3>
                <p className="mt-2 text-sm leading-relaxed text-zinc-400">
                  The login menu opens on offline login. Play offline-mode
                  servers and singleplayer without a Microsoft account.
                  Microsoft and authlib-injector still exist for people who
                  want them.
                </p>
                <div className="mt-4 font-mono text-[11px] text-zinc-600">
                  <span className="text-accent/50">→</span> Offline · Microsoft · authlib-injector
                </div>
              </div>
            </Reveal>
          </div>

          {/* Row 2: three compact cards */}
          <div className="mt-4 grid gap-4 md:grid-cols-3">
            {[
              {
                icon: IconLightning,
                title: "Honest performance",
                body: "Memory scales with your device instead of a flat cap. A tuned G1 baseline keeps frame times steady on phone-class hardware.",
              },
              {
                icon: IconGlobe,
                title: "Official sources only",
                body: "Game files download from Mojang's own endpoints with SHA1 verification. No mirrors, no repacked jars.",
              },
              {
                icon: IconCpu,
                title: "Java 25, MC 26.3",
                body: "The Java 25 runtime ships inside the Android APK. Linux finds a suitable JDK on your system.",
              },
            ].map((feature, index) => (
              <Reveal key={feature.title} delay={index * 0.06} className="h-full">
                <div className="lift h-full rounded-xl bg-panel p-6 hover:bg-panel-2">
                  <feature.icon size={20} className="text-accent/60" />
                  <h3 className="mt-4 text-sm font-medium">{feature.title}</h3>
                  <p className="mt-2 text-[13px] leading-relaxed text-zinc-400">
                    {feature.body}
                  </p>
                </div>
              </Reveal>
            ))}
          </div>
        </section>

        {/* ─── Benchmarks ─── */}
        <section id="bench" className="mx-auto max-w-5xl px-5 py-16 md:py-28">
          <Reveal>
            <h2 className="text-2xl font-semibold tracking-[-0.02em] md:text-[1.75rem]">
              Install time, measured
            </h2>
            <p className="mt-3 max-w-xl text-sm text-zinc-400">
              Fresh MC 26.3 installs run back to back on the same machine, the
              same manifest, the same 5,199 files. These are the numbers from
              those runs, recorded in the changelog.
            </p>
          </Reveal>
          <div className="mt-10 grid gap-4 md:grid-cols-2">
            {BENCH.map((group, index) => (
              <Reveal key={group.title} delay={index * 0.08} className="h-full">
                <div className="lift flex h-full flex-col rounded-xl bg-panel p-7">
                  <div className="flex items-baseline justify-between gap-4">
                    <h3 className="text-[15px] font-medium">{group.title}</h3>
                    <span className="font-mono text-[10px] tracking-wide text-zinc-600 uppercase">
                      lower is better
                    </span>
                  </div>
                  <div className="mt-6 space-y-5">
                    {group.rows.map((row) => (
                      <BenchBar
                        key={row.label}
                        label={row.label}
                        seconds={row.seconds}
                        accent={row.accent}
                      />
                    ))}
                  </div>
                  <p className="mt-6 text-[13px] leading-relaxed text-zinc-500">
                    {group.note}
                  </p>
                </div>
              </Reveal>
            ))}
          </div>
        </section>

        {/* ─── Screens ─── */}
        <section id="screens" className="mx-auto max-w-5xl px-5 py-16 md:py-28">
          <Reveal>
            <h2 className="text-2xl font-semibold tracking-[-0.02em] md:text-[1.75rem]">
              Real screens, real installs
            </h2>
            <p className="mt-3 max-w-xl text-sm text-zinc-400">
              Captured from the Android app running on an emulator. The dark
              theme is the theme.
            </p>
          </Reveal>
          <div className="mt-10 grid gap-4">
            {/* Lead frame: full width, the home screen with 26.3 installed */}
            <Reveal>
              <button
                type="button"
                onClick={() => setOpenShot(SHOTS[0])}
                className="shot-frame block w-full overflow-hidden rounded-xl border border-white/[0.05] bg-panel text-left"
              >
                <img
                  src={SHOTS[0].src}
                  alt={SHOTS[0].alt}
                  loading="eager"
                  className="w-full aspect-[2400/1080] object-cover"
                />
                <span className="flex items-center justify-between border-t border-white/[0.04] px-5 py-3.5">
                  <span className="text-sm text-zinc-200">{SHOTS[0].label}</span>
                  <span className="font-mono text-[10px] text-zinc-600">
                    Android 15 · 2400×1080 · offline account
                  </span>
                </span>
              </button>
            </Reveal>
            <div className="grid gap-4 md:grid-cols-3">
              {SHOTS.slice(1).map((shot, index) => (
                <Reveal key={shot.src} delay={0.06 + index * 0.06}>
                  <button
                    type="button"
                    onClick={() => setOpenShot(shot)}
                    className="shot-frame block w-full overflow-hidden rounded-xl border border-white/[0.05] bg-panel text-left"
                  >
                    <img
                      src={shot.src}
                      alt={shot.alt}
                      loading="lazy"
                      className="w-full aspect-[2400/1080] object-cover"
                    />
                    <span className="flex items-center justify-between border-t border-white/[0.04] px-4 py-3">
                      <span className="text-[13px] text-zinc-300">{shot.label}</span>
                      <span className="font-mono text-[10px] text-zinc-600">26.3</span>
                    </span>
                  </button>
                </Reveal>
              ))}
            </div>
          </div>
        </section>

        {/* ─── Download ─── */}
        <section id="download" className="mx-auto max-w-5xl px-5 py-16 md:py-28">
          <Reveal>
            <h2 className="text-2xl font-semibold tracking-[-0.02em] md:text-[1.75rem]">
              Get Basalt
            </h2>
          </Reveal>
          <div className="mt-10 grid gap-4 lg:grid-cols-2">
            <Reveal>
              <div className="flex h-full flex-col rounded-xl bg-panel p-8">
                <IconDownload size={24} className="text-accent/60" />
                <h3 className="mt-5 text-xl font-semibold">Android</h3>
                <p className="mt-2 flex-1 text-sm leading-relaxed text-zinc-400">
                  Universal APK with the Java 25 runtime inside. Sideload it:
                  allow install-from-unknown-sources, done.
                </p>
                <div className="mt-6 flex flex-wrap gap-3">
                  <a
                    href="/dl/Basalt-2.0.0-universal.apk"
                    className="inline-flex items-center gap-2 rounded-lg bg-accent px-5 py-2.5 text-sm font-semibold text-black transition-all active:scale-[0.98] hover:brightness-110"
                  >
                    Get the APK
                  </a>
                  <a
                    href="/dl/Basalt-2.0.0-arm64-v8a.apk"
                    className="inline-flex items-center gap-2 rounded-lg border border-white/[0.08] px-5 py-2.5 text-sm font-medium text-zinc-300 transition-colors hover:text-white"
                  >
                    arm64 only
                  </a>
                </div>
                <p className="mt-4 font-mono text-[10px] text-zinc-600">
                  400 MB universal · 182 MB arm64 · Android 8.0+
                </p>
              </div>
            </Reveal>
            <Reveal delay={0.08}>
              <div className="flex h-full flex-col rounded-xl bg-panel p-8">
                <IconTerminal size={24} className="text-accent/60" />
                <h3 className="mt-5 text-xl font-semibold">Linux</h3>
                <p className="mt-2 flex-1 text-sm leading-relaxed text-zinc-400">
                  One portable AppImage with a GUI window and a CLI inside.
                  Needs Java 25 and internet.
                </p>
                <div className="mt-6">
                  <a
                    href="/dl/Basalt-2.0.0-x86_64.AppImage"
                    className="inline-flex items-center gap-2 rounded-lg bg-accent px-5 py-2.5 text-sm font-semibold text-black transition-all active:scale-[0.98] hover:brightness-110"
                  >
                    Get the AppImage
                  </a>
                </div>
                <div className="mt-4 font-mono text-[10px] text-zinc-600">
                  6.6 MB · x86_64 · chmod +x required
                </div>
                <div className="mt-4 rounded-lg bg-black/30 px-3 py-2.5">
                  <p className="font-mono text-[10px] leading-relaxed break-all text-zinc-600">
                    sha256 {APPIMAGE_SHA256}
                  </p>
                </div>
              </div>
            </Reveal>
          </div>

          <Reveal delay={0.12}>
            <div className="mt-4 flex flex-wrap items-center gap-x-6 gap-y-3 rounded-xl border border-white/[0.06] bg-panel px-7 py-5 text-sm text-zinc-400">
              <span className="text-zinc-300">Also here:</span>
              <a
                href={REPO}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-2 transition-colors hover:text-white"
              >
                <IconGithub size={16} />
                Source, GPL-3, builds from a clean clone
              </a>
              <a
                href={RELEASES}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-2 transition-colors hover:text-white"
              >
                <IconDownload size={16} />
                Release notes and checksums
              </a>
            </div>
          </Reveal>

          <Reveal delay={0.1}>
            <div className="mt-4 rounded-xl bg-panel p-8">
              <h3 className="text-[15px] font-semibold">What a launch does</h3>
              <div className="mt-4 grid gap-3 text-sm text-zinc-400 md:grid-cols-2">
                {CHECKS.map((check) => (
                  <div key={check} className="flex items-start gap-2.5">
                    <IconCheck size={16} className="mt-0.5 shrink-0 text-accent/60" />
                    <span>{check}</span>
                  </div>
                ))}
              </div>
            </div>
          </Reveal>
        </section>

        {/* ─── Honesty ─── */}
        <section className="mx-auto max-w-5xl px-5 pb-16 md:pb-24">
          <Reveal>
            <div className="rounded-xl border border-accent/[0.08] bg-panel p-8">
              <h3 className="text-[15px] font-semibold">The honest parts</h3>
              <p className="mt-3 max-w-3xl text-sm leading-relaxed text-zinc-400">
                Basalt is a GPL-3 fork of ZalithLauncher 2. It is not affiliated
                with Mojang Studios or Microsoft, and it ships none of their
                code: the game downloads from their own servers at install
                time. Online servers that check accounts need a real account;
                offline accounts exist for offline play and offline-mode
                servers. The Rust launch engine and this site's design are
                first-party work.
              </p>
            </div>
          </Reveal>
        </section>
      </main>

      {/* Footer */}
      <footer className="relative z-10 border-t border-white/[0.04] bg-black/30">
        <div className="mx-auto grid max-w-5xl gap-10 px-5 py-16 md:grid-cols-4">
          <div className="md:col-span-2">
            <div className="flex items-center gap-2.5">
              <Mark size={24} />
              <span className="text-sm font-semibold">Basalt</span>
            </div>
            <p className="mt-4 max-w-sm text-sm leading-relaxed text-zinc-500">
              A Minecraft Java Edition launcher for Android and Linux. Fork of
              ZalithLauncher 2, GPL-3. Built by Leonxlnx.
            </p>
          </div>
          <div>
            <h4 className="text-[13px] font-semibold">Project</h4>
            <ul className="mt-3 space-y-2 text-sm text-zinc-500">
              <li>
                <a
                  href={REPO}
                  target="_blank"
                  rel="noreferrer"
                  className="transition-colors hover:text-zinc-200"
                >
                  Source
                </a>
              </li>
              <li>
                <a
                  href={RELEASES}
                  target="_blank"
                  rel="noreferrer"
                  className="transition-colors hover:text-zinc-200"
                >
                  Releases
                </a>
              </li>
              <li>
                <a
                  href={`${REPO}/blob/main/LICENSE`}
                  target="_blank"
                  rel="noreferrer"
                  className="transition-colors hover:text-zinc-200"
                >
                  License
                </a>
              </li>
              <li>
                <a
                  href="https://github.com/ZalithLauncher/ZalithLauncher2"
                  target="_blank"
                  rel="noreferrer"
                  className="transition-colors hover:text-zinc-200"
                >
                  Upstream
                </a>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="text-[13px] font-semibold">Stack</h4>
            <ul className="mt-3 space-y-2 font-mono text-[11px] text-zinc-600">
              <li>Rust · engine + CLI</li>
              <li>Kotlin · Compose app</li>
              <li>GPL-3.0-or-later</li>
            </ul>
          </div>
        </div>
        <div className="border-t border-white/[0.04]">
          <div className="mx-auto flex max-w-5xl flex-wrap items-center justify-between gap-2 px-5 py-5 font-mono text-[11px] text-zinc-700">
            <span>Not affiliated with Mojang Studios or Microsoft.</span>
            <span>v2.0.0 · 2026</span>
          </div>
        </div>
      </footer>

      {openShot && <Lightbox shot={openShot} onClose={() => setOpenShot(null)} />}
    </div>
  );
}

export default App;
