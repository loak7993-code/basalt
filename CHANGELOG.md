# Changelog

All notable changes to this fork are recorded here. Upstream history lives in
the git lineage of the fork point.

## 2.0.0 - 2026-10-09

Update (2026-10-10): the Android download engine now runs in Rust too. The
`basalt_engine` download module (shared keep-alive agent, bounded workers,
60s timeouts, 3-attempt retries with URL rotation, SHA1 verification,
byte-accurate progress) is bound over JNI with a job-based API
(start/progress/wait) and replaces the Kotlin `BatchDownloader` transfer path
for version installs, with the Kotlin engine kept as fallback. Measured on
the emulator with fresh 26.3 installs back to back: Kotlin engine 120s, Rust
engine 37s — 3.2x faster. Verified: no fallback warnings during the Rust-path
launch, install completes with zero failures.

- The launch engine is now Rust (`engine-rs`, crate `basalt_engine`): version
  manifest parsing, Mojang library rule evaluation, classpath construction,
  memory and GC policies, argument assembly, and an exec strategy interface,
  all as first-party Rust with serde models.
- The full Kotlin engine test suite is ported and green (`cargo test`, 14
  tests), including a parity test proving the Rust plan matches the args the
  app actually launched with on-device (heap 1637M, tuned G1 baseline,
  ActiveProcessorCount=4).
- The Android app binds to the Rust engine over JNI
  (`libbasalt_engine.so`, x86_64 + arm64, NDK 27, minSdk 26): the GC policy
  and the device-proportional memory default now run through
  `libbasalt_engine.so` in both the main and `:game` processes.
- If the native library ever fails to load, every call degrades to the
  identical Kotlin policy and logs the failure once, so a broken build cannot
  change launch behavior.
- Verified on the emulator: fresh launch through the `:game` process with zero
  fallback warnings and identical args through the Rust path.
- The Kotlin `:engine` module remains as the reference implementation and its
  tests still run in Gradle builds.

## 1.2.0 - 2026-10-09

- New `:engine` module, Basalt's own launch engine: version manifest parsing
  (real 26.3 shape), Mojang library rule evaluation (os/arch/features),
  classpath construction, memory and GC policies, full JVM and game argument
  assembly with placeholder substitution, quick play handling, and an exec
  strategy interface with a reference child-process implementation.
- 23 unit tests run against the real Mojang 26.3 manifest as the fixture:
  platform filtering, natives selection, policy overrides, placeholder
  resolution, command formation, real exit codes.
- The app's launcher now applies its GC policy through the engine's
  `gcPolicyFromArgs` + `applyTo`, so the tested engine logic is the single
  source of truth for launch behavior instead of an inline copy.
- 26.3 detail: the manifest now ships its own JVM defaults
  (`default-user-jvm`: heap, ZGC on desktop, CompactObjectHeaders,
  AlwaysPreTouch). The engine evaluates them, then the caller's policy
  replaces heap and collector selection; object-layout flags survive.

## 1.1.0 - 2026-10-09

- Empty home now carries its own action: when no versions are installed, the
  card grid area shows a compact empty state with a working "Download a
  version" button that opens the download screen.
- Version names on home cards and in the version row render in Geist Mono,
  matching the download list.
- Verified the game console persists to disk per version (`versions/<name>/
  Basalt/`); the JVM args section is now inspectable after a crash instead of
  living only in the on-screen log view.

## 1.0.0 - 2026-10-09

Fork of ZalithLauncher2, upstream 2.6.1 (commit d93722e).

- Rebrand to Basalt: new application id `dev.leonxlnx.basalt`, new name, icon,
  release keystore (`basalt.jks`, committed for side-load distribution,
  password documented in the README).
- Locked dark look: new Basalt palette (near-black canvas `#050505`-`#0A0A0A`,
  panel ramp `#101012`-`#17181A`, single emerald accent), set as the default
  color theme; dark mode on by default.
- Typography: Geist Sans for UI text, Geist Mono for numeric and code surfaces,
  both bundled (OFL). Icon family swapped wholesale to Phosphor bold (MIT).
- Offline accounts are now the first entry in the login menu.
- Memory allocation defaults now scale with the device (about 55% of total RAM,
  floor 1536 MB within device limits) instead of the upstream tier table that
  capped every 6 GB+ device at 2048 MB.
- The game JVM gets a tuned G1 baseline when the user has not configured a
  garbage collector (`-XX:+UseG1GC`, `G1NewSizePercent=20`,
  `G1ReservePercent=20`, `MaxGCPauseMillis=40`, `G1HeapRegionSize=16M`,
  `-XX:+DisableExplicitGC`).
- Update and project info endpoints repointed to the fork's repository; the
  upstream donation and translation-service links removed from the fork.
- Festival effects (seasonal overlays) disabled by default.
