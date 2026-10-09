import { motion } from "motion/react";
import {
  IconCheck,
  IconCpu,
  IconDownload,
  IconGlobe,
  IconLightning,
  IconPlay,
  IconTerminal,
  IconUsers,
} from "./components/icons";

const EASE = [0.16, 1, 0.3, 1] as const;

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
      initial={{ opacity: 0, y: 24 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-80px" }}
      transition={{ duration: 0.5, delay, ease: EASE }}
      className={className}
    >
      {children}
    </motion.div>
  );
}

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

const NAV = [
  { label: "Features", href: "#features" },
  { label: "Screens", href: "#screens" },
  { label: "Download", href: "#download" },
];

const FEATURES = [
  {
    icon: IconTerminal,
    title: "Rust launch engine",
    body: "The same engine crate drives Android and Linux. It parses the official manifest, filters libraries by platform, and assembles every JVM and game argument.",
  },
  {
    icon: IconUsers,
    title: "Offline accounts first",
    body: "The login menu opens on offline login. Play offline-mode servers and singleplayer without a Microsoft account. Microsoft login still exists for people who want it.",
  },
  {
    icon: IconLightning,
    title: "Honest performance defaults",
    body: "Memory scales with your device instead of a flat cap, and a tuned G1 baseline keeps frame times steady on phone-class hardware.",
  },
  {
    icon: IconGlobe,
    title: "Official sources only",
    body: "Game files download from Mojang's own endpoints with SHA1 verification. No mirrors, no repacked jars, no bundled game code.",
  },
  {
    icon: IconCpu,
    title: "Java 25, MC 26.3",
    body: "The Java 25 runtime ships inside the Android APK. Linux finds a suitable JDK on your system and refuses anything older than the game needs.",
  },
  {
    icon: IconPlay,
    title: "Renderers your GPU deserves",
    body: "Android picks from gl4es, ANGLE, Zink, VirGL and friends per device. Linux runs the stock LWJGL pipeline your desktop already supports.",
  },
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
];

function App() {
  return (
    <div className="relative min-h-screen bg-canvas text-zinc-100">
      <div className="glow-haze" />

      {/* Nav */}
      <header className="sticky top-0 z-40 border-b border-white/5 backdrop-blur-md bg-black/40">
        <div className="mx-auto flex h-14 max-w-6xl items-center justify-between px-5">
          <a href="#top" className="flex items-center gap-2.5">
            <Mark size={26} />
            <span className="text-[15px] font-semibold tracking-tight">Basalt</span>
          </a>
          <nav className="flex items-center gap-1 text-sm text-zinc-400">
            {NAV.map((item) => (
              <a
                key={item.href}
                href={item.href}
                className="rounded-lg px-3 py-1.5 transition-colors hover:text-zinc-100 hover:bg-white/5"
              >
                {item.label}
              </a>
            ))}
            <a
              href="https://github.com/Leonxlnx/Basalt"
              className="ml-2 rounded-lg border border-white/10 px-3 py-1.5 text-zinc-200 transition-colors hover:border-accent/40 hover:text-white"
            >
              GitHub
            </a>
          </nav>
        </div>
      </header>

      <main id="top" className="relative z-10">
        {/* Hero */}
        <section className="mx-auto max-w-6xl px-5 pt-24 pb-20 md:pt-36 md:pb-28">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.55, ease: EASE }}
            className="max-w-3xl"
          >
            <p className="font-mono text-xs tracking-widest text-accent/80 uppercase">
              GPL-3 · Rust engine · Offline first
            </p>
            <h1 className="mt-4 text-4xl font-semibold leading-[1.05] tracking-tight md:text-6xl">
              Minecraft Java Edition,
              <br />
              <span className="text-zinc-400">launched by Rust.</span>
            </h1>
            <p className="mt-6 max-w-xl text-base leading-relaxed text-zinc-400 md:text-lg">
              Basalt installs the game from Mojang's own servers and launches it
              on Android and Linux. No account wall for offline play, no
              repacked jars, no fake performance numbers.
            </p>
            <div className="mt-8 flex flex-wrap items-center gap-3">
              <a
                href="#download"
                className="inline-flex items-center gap-2 rounded-lg bg-accent px-5 py-2.5 text-sm font-semibold text-black transition-transform active:scale-[0.98] hover:bg-emerald-300"
              >
                <IconDownload size={18} />
                Download
              </a>
              <a
                href="#features"
                className="inline-flex items-center gap-2 rounded-lg border border-white/10 px-5 py-2.5 text-sm font-medium text-zinc-200 transition-colors hover:border-accent/40 hover:text-white"
              >
                What it does
              </a>
            </div>
            <div className="mt-8 flex flex-wrap gap-x-6 gap-y-2 font-mono text-xs text-zinc-500">
              <span>MC 26.3 · current release</span>
              <span>Java 25 bundled on Android</span>
              <span>5,199-file install, SHA1 verified</span>
            </div>
          </motion.div>
        </section>

        {/* Features */}
        <section id="features" className="mx-auto max-w-6xl px-5 py-16 md:py-24">
          <Reveal>
            <h2 className="text-2xl font-semibold tracking-tight md:text-3xl">
              Built different, on purpose
            </h2>
            <p className="mt-3 max-w-2xl text-zinc-400">
              A fork of ZalithLauncher 2 with the launch engine rewritten in Rust
              and the whole product layer rebuilt.
            </p>
          </Reveal>
          <div className="mt-10 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {FEATURES.map((feature, index) => (
              <Reveal
                key={feature.title}
                delay={index * 0.06}
                className="h-full"
              >
                <div className="group h-full rounded-xl bg-panel p-6 transition-colors hover:bg-panel-2">
                  <feature.icon size={22} className="text-accent/70" />
                  <h3 className="mt-4 font-medium">{feature.title}</h3>
                  <p className="mt-2 text-sm leading-relaxed text-zinc-400">
                    {feature.body}
                  </p>
                </div>
              </Reveal>
            ))}
          </div>
        </section>

        {/* Screens */}
        <section id="screens" className="mx-auto max-w-6xl px-5 py-16 md:py-24">
          <Reveal>
            <h2 className="text-2xl font-semibold tracking-tight md:text-3xl">
              Real screens, real installs
            </h2>
            <p className="mt-3 max-w-2xl text-zinc-400">
              Captured from the Android app running on an emulator. The dark
              theme is the theme: near-black canvas, one emerald accent.
            </p>
          </Reveal>
          <div className="mt-10 grid gap-4 md:grid-cols-2">
            {SHOTS.map((shot, index) => (
              <Reveal key={shot.src} delay={index * 0.06}>
                <figure className="overflow-hidden rounded-xl border border-white/5 bg-panel">
                  <img
                    src={shot.src}
                    alt={shot.alt}
                    loading="lazy"
                    className="w-full"
                  />
                  <figcaption className="flex items-center justify-between px-4 py-3 text-sm text-zinc-400">
                    <span>{shot.label}</span>
                    <span className="font-mono text-xs text-zinc-600">
                      Android 15 · 2400×1080
                    </span>
                  </figcaption>
                </figure>
              </Reveal>
            ))}
          </div>
        </section>

        {/* Download */}
        <section id="download" className="mx-auto max-w-6xl px-5 py-16 md:py-24">
          <div className="grid gap-4 lg:grid-cols-2">
            <Reveal>
              <div className="flex h-full flex-col rounded-xl bg-panel p-8">
                <IconDownload size={24} className="text-accent/70" />
                <h3 className="mt-4 text-xl font-semibold">Android</h3>
                <p className="mt-2 text-sm leading-relaxed text-zinc-400">
                  Universal APK with the Java 25 runtime inside. Sideload it:
                  allow install-from-unknown-sources, done.
                </p>
                <div className="mt-6">
                  <a
                    href="https://github.com/Leonxlnx/Basalt/releases"
                    className="inline-flex items-center gap-2 rounded-lg bg-accent px-5 py-2.5 text-sm font-semibold text-black transition-transform active:scale-[0.98] hover:bg-emerald-300"
                  >
                    Get the APK
                  </a>
                </div>
              </div>
            </Reveal>
            <Reveal delay={0.08}>
              <div className="flex h-full flex-col rounded-xl bg-panel p-8">
                <IconTerminal size={24} className="text-accent/70" />
                <h3 className="mt-4 text-xl font-semibold">Linux</h3>
                <p className="mt-2 text-sm leading-relaxed text-zinc-400">
                  One portable AppImage. Needs Java 25 and internet: it
                  downloads the game from Mojang, then plays it.
                </p>
                <div className="mt-6 rounded-lg bg-black/50 p-4 font-mono text-sm text-zinc-300">
                  <span className="text-zinc-600">$</span> ./Basalt-x86_64.AppImage account myname
                  <br />
                  <span className="text-zinc-600">$</span> ./Basalt-x86_64.AppImage install 26.3
                  <br />
                  <span className="text-zinc-600">$</span> ./Basalt-x86_64.AppImage play 26.3
                </div>
              </div>
            </Reveal>
          </div>

          <Reveal delay={0.1}>
            <div className="mt-4 rounded-xl bg-panel p-8">
              <h3 className="text-lg font-semibold">What a launch does</h3>
              <ul className="mt-4 grid gap-3 text-sm text-zinc-400 md:grid-cols-2">
                {CHECKS.map((check) => (
                  <li key={check} className="flex items-start gap-2.5">
                    <IconCheck size={18} className="mt-0.5 shrink-0 text-accent/70" />
                    <span>{check}</span>
                  </li>
                ))}
              </ul>
            </div>
          </Reveal>
        </section>

        {/* Honesty */}
        <section className="mx-auto max-w-6xl px-5 pb-16 md:pb-24">
          <Reveal>
            <div className="rounded-xl border border-white/5 bg-panel p-8">
              <h3 className="text-lg font-semibold">The honest parts</h3>
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
      <footer className="relative z-10 border-t border-white/5 bg-black/30">
        <div className="mx-auto grid max-w-6xl gap-10 px-5 py-14 md:grid-cols-4">
          <div className="md:col-span-2">
            <div className="flex items-center gap-2.5">
              <Mark size={26} />
              <span className="text-[15px] font-semibold">Basalt</span>
            </div>
            <p className="mt-4 max-w-sm text-sm leading-relaxed text-zinc-500">
              A Minecraft Java Edition launcher for Android and Linux. Fork of
              ZalithLauncher 2, GPL-3. Built by Leonxlnx.
            </p>
          </div>
          <div>
            <h4 className="text-sm font-semibold">Project</h4>
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
            <h4 className="text-sm font-semibold">Stack</h4>
            <ul className="mt-3 space-y-2 font-mono text-xs text-zinc-500">
              <li>Rust · engine + CLI</li>
              <li>Kotlin · Compose app</li>
              <li>GPL-3.0-or-later</li>
            </ul>
          </div>
        </div>
        <div className="border-t border-white/5">
          <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-2 px-5 py-5 font-mono text-xs text-zinc-600">
            <span>Not affiliated with Mojang Studios or Microsoft.</span>
            <span>v2.0.0 · {new Date().getFullYear()}</span>
          </div>
        </div>
      </footer>
    </div>
  );
}

export default App;
