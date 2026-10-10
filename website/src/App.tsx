import { useEffect, useRef, useState } from "react";
import { motion, useInView } from "motion/react";
import { useTerminalTranscript } from "./useTerminal";
import {
  IconCheck,
  IconCpu,
  IconDownload,
  IconGlobe,
  IconLightning,
  IconTerminal,
  IconUsers,
} from "./components/icons";

const EASE = [0.16, 1, 0.3, 1] as const;

function Mark({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 108 108" aria-hidden="true">
      <rect width="108" height="108" rx="22" fill="#0A0A0C" />
      <path d="M36,58 46,54 46,72 36,72 Z M34,74.5 74,74.5 74,76.5 34,76.5 Z" fill="#2EA982" />
      <path d="M48,44 58,40 58,72 48,72 Z" fill="#4ADE9C" />
      <path d="M60,56 70,52 70,72 60,72 Z" fill="#2EA982" />
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

/* A terminal that behaves like one: typing, output, a live progress bar. */
function Terminal() {
  const { lines, typingLine, progress } = useTerminalTranscript();
  const scrollRef = useRef<HTMLDivElement>(null);

  // Keep the newest line in view as the transcript grows.
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines.length, typingLine, progress]);

  const toneClass = (tone?: string) =>
    tone === "accent" ? "text-accent" : tone === "plain" ? "text-zinc-200" : "text-zinc-500";

  return (
    <div className="relative rounded-xl border border-white/[0.06] bg-[#08080a] overflow-hidden terminal-shell">
      <div className="flex items-center gap-1.5 px-4 py-2.5 border-b border-white/[0.06]">
        <div className="w-2.5 h-2.5 rounded-full bg-[#3a3a3c]" />
        <div className="w-2.5 h-2.5 rounded-full bg-[#3a3a3c]" />
        <div className="w-2.5 h-2.5 rounded-full bg-[#3a3a3c]" />
        <span className="ml-2 font-mono text-[11px] text-zinc-600">steve@basalt — zsh</span>
        <span className="ml-auto font-mono text-[10px] text-zinc-700">26.3</span>
      </div>
      <div
        ref={scrollRef}
        className="h-[268px] overflow-hidden p-4 font-mono text-[12.5px] leading-[1.75] terminal-body"
      >
        {lines.map((line, i) => (
          <div key={i} className={toneClass(line.tone)}>
            {line.text || "\u00A0"}
          </div>
        ))}
        {typingLine && (
          <div className="text-zinc-200">
            <span className="text-accent">{typingLine.slice(0, 2)}</span>
            {typingLine.slice(2)}
            <span className="cursor-blink text-accent">▊</span>
          </div>
        )}
        {progress && (
          <div className="text-zinc-500">
            <span className="text-accent">{progress.filled}</span>
            <span className="text-zinc-600">{progress.tail}</span>
            {"  "}
            {progress.label}
          </div>
        )}
        {!typingLine && !progress && (
          <div className="text-zinc-600">
            <span className="text-accent">❯</span> <span className="cursor-blink">▊</span>
          </div>
        )}
      </div>
      {/* soft scanline glow at the bottom */}
      <div className="pointer-events-none absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-[#08080a] to-transparent" />
    </div>
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
  { label: "Screens", href: "#screens" },
  { label: "Download", href: "#download" },
];

const SHOTS = [
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

function App() {
  return (
    <div className="relative min-h-screen bg-canvas text-zinc-100">
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
              href="https://github.com/Leonxlnx/Basalt"
              className="ml-2 rounded-lg border border-white/[0.08] px-3 py-1.5 text-zinc-300 transition-colors hover:text-white hover:border-white/20"
            >
              Source
            </a>
          </nav>
        </div>
      </header>

      <main id="top" className="relative z-10">
        {/* ─── Hero ─── */}
        <section
          className="hero-with-spot relative mx-auto max-w-5xl px-5 pt-32 pb-16 md:pt-44 md:pb-24"
          onPointerMove={(e) => {
            const target = e.currentTarget;
            const rect = target.getBoundingClientRect();
            target.style.setProperty("--spot-x", `${e.clientX - rect.left}px`);
            target.style.setProperty("--spot-y", `${e.clientY - rect.top}px`);
          }}
        >
          <div className="spotlight" aria-hidden="true" />
          <div className="relative grid items-center gap-12 lg:grid-cols-[1fr_1fr]">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.55, ease: EASE }}
            >
              <p className="font-mono text-[11px] tracking-[0.2em] text-accent/70 uppercase">
                GPL-3 · Rust engine · Offline first
              </p>
              <h1 className="mt-5 text-[2.5rem] font-semibold leading-[1.02] tracking-[-0.03em] md:text-[3.6rem]">
                Minecraft Java Edition,
                <br />
                <span className="text-zinc-500">launched by Rust.</span>
              </h1>
              <p className="mt-6 max-w-md text-[15px] leading-relaxed text-zinc-400">
                Basalt installs the game from Mojang's own servers and launches
                it on Android and Linux. No account wall for offline play.
              </p>
              <div className="mt-8 flex flex-wrap items-center gap-3">
                <a
                  href="#download"
                  className="inline-flex items-center gap-2 rounded-lg bg-accent px-5 py-2.5 text-sm font-semibold text-black transition-all active:scale-[0.98] hover:bg-emerald-300"
                >
                  <IconDownload size={16} />
                  Download
                </a>
                <a
                  href="#features"
                  className="inline-flex items-center gap-2 rounded-lg border border-white/[0.08] px-5 py-2.5 text-sm font-medium text-zinc-300 transition-colors hover:text-white hover:border-white/20"
                >
                  How it works
                </a>
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.55, delay: 0.15, ease: EASE }}
            >
              <Terminal />
            </motion.div>
          </div>

          {/* Stats bar */}
          <Reveal delay={0.2} className="mt-16">
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
              <figure className="shot-frame overflow-hidden rounded-xl border border-white/[0.05] bg-panel">
                <img
                  src={SHOTS[0].src}
                  alt={SHOTS[0].alt}
                  loading="eager"
                  className="w-full aspect-[2400/1080] object-cover"
                />
                <figcaption className="flex items-center justify-between border-t border-white/[0.04] px-5 py-3.5">
                  <span className="text-sm text-zinc-200">{SHOTS[0].label}</span>
                  <span className="font-mono text-[10px] text-zinc-600">
                    Android 15 · 2400×1080 · offline account
                  </span>
                </figcaption>
              </figure>
            </Reveal>
            <div className="grid gap-4 md:grid-cols-3">
              {SHOTS.slice(1).map((shot, index) => (
                <Reveal key={shot.src} delay={0.06 + index * 0.06}>
                  <figure className="shot-frame overflow-hidden rounded-xl border border-white/[0.05] bg-panel">
                    <img
                      src={shot.src}
                      alt={shot.alt}
                      loading="lazy"
                      className="w-full aspect-[2400/1080] object-cover"
                    />
                    <figcaption className="flex items-center justify-between border-t border-white/[0.04] px-4 py-3">
                      <span className="text-[13px] text-zinc-300">{shot.label}</span>
                      <span className="font-mono text-[10px] text-zinc-600">26.3</span>
                    </figcaption>
                  </figure>
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
                    className="inline-flex items-center gap-2 rounded-lg bg-accent px-5 py-2.5 text-sm font-semibold text-black transition-all active:scale-[0.98] hover:bg-emerald-300"
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
                    className="inline-flex items-center gap-2 rounded-lg bg-accent px-5 py-2.5 text-sm font-semibold text-black transition-all active:scale-[0.98] hover:bg-emerald-300"
                  >
                    Get the AppImage
                  </a>
                </div>
                <div className="mt-4 font-mono text-[10px] text-zinc-600">
                  6.6 MB · x86_64 · chmod +x required
                </div>
              </div>
            </Reveal>
          </div>

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
                <a href="https://github.com/Leonxlnx/Basalt" className="transition-colors hover:text-zinc-200">
                  Source
                </a>
              </li>
              <li>
                <a href="https://github.com/Leonxlnx/Basalt/releases" className="transition-colors hover:text-zinc-200">
                  Releases
                </a>
              </li>
              <li>
                <a href="https://github.com/ZalithLauncher/ZalithLauncher2" className="transition-colors hover:text-zinc-200">
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
    </div>
  );
}

export default App;
