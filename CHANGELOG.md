# Changelog

All notable changes to this fork are recorded here. Upstream history lives in
the git lineage of the fork point.

## Unreleased

Project and site work published after the 2.0.0 tag:

- CurseForge 403 fix: `api.curseforge.com` rejects every request that carries
  no `x-api-key`, and this build ships no key, so mods, shader packs, resource
  packs and modpacks all failed with `Forbidden: API Key missing or invalid`.
  `mirroredCurseForgeSource()` now leads with the keyless MCIM mirror when no
  key is configured and keeps the official source in front when one is.
  Modrinth was never affected.

- Published at `github.com/loak7993-code/basalt` (public, GPL-3). Build
  output, `node_modules`, the served site bundle, the release keystore and the
  share log are no longer tracked, and the release keystore passwords were
  removed from `gradle.properties`. History was rewritten to purge those paths,
  which took the repository from 1.8 GB to 443 MB. The shipped binaries are
  release assets, not commits.
- Tag `v2.0.0` carries the arm64 APK, the universal APK and the AppImage, with
  release notes and the verification record.
- Linux CLI build fix: `EngineOptions` gained `ignore_default_user_jvm`,
  `strings_only` and `substitute_classpath` for the Android launch-plan work
  and `cli/src/launch.rs` was never updated, so `cargo build` in `cli/` failed
  with E0063. Desktop now sets all three to false, which is the behaviour the
  shipped binary already had.
- Dead code removed: the CLI's unused `download_with_retry` helper and its
  `RETRY_ATTEMPTS` constant (superseded by the inline URL rotation in
  `run_all`), plus unused imports in the engine.
- Engine and CLI are now `cargo fmt` clean and `cargo clippy -D warnings`
  clean. GitHub Actions runs both test suites, both lint gates and the site
  build on every push.
- Site: benchmark section with the measured install times (Android 120s Kotlin
  engine vs 37s Rust engine, Linux 30s vs 13s), a screenshot lightbox, a
  copyable one-line Linux install command, the AppImage sha256 next to the
  download, OG card, favicon, skip link and visible focus rings.
- Site fix: the hero's flex child had `min-width: auto`, so the headline
  overflowed its column and the section's `overflow-hidden` clipped it on
  mobile. Fixed with `min-w-0` and a responsive type scale.

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
