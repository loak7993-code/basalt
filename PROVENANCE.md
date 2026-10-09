# Provenance

Basalt is a fork of ZalithLauncher 2 by MovTery and contributors, upstream
repository https://github.com/ZalithLauncher/ZalithLauncher2, fork point
upstream version 2.6.1 (commit d93722e, October 2026).

## License lineage

- The entire runtime core is upstream GPL-3.0 code: the Android JVM launch
  pipeline, the bundled bionic Java runtimes (8/17/21/25), the patched LWJGL
  builds (3.3.3, 3.4.1), the GL renderers and their plugin system, the input
  bridge and native libraries, the Terracotta multiplayer client, and the
  overall application architecture. Those parts remain GPL-3.0 under the terms
  of the included LICENSE, and this fork's code is GPL-3.0 for the same reason.
- The fork's product-layer changes (theme, typography, icon set, branding,
  account flow defaults, memory defaults, GC baseline) are new work in this
  repository, also GPL-3.0.

### The engine boundary (Basalt 2.0.0)

Basalt's launch engine is Rust: `engine-rs/` (crate `basalt_engine`, GPL-3).
It owns, as first-party Rust code with tests against the real Mojang 26.3
manifest:

- version manifest parsing (arguments, libraries, rules, logging config,
  `default-user-jvm`) via serde
- library rule evaluation per platform (os, arch, features)
- classpath construction and natives selection
- memory policy and GC policy (the tuned phone-class G1 baseline)
- full JVM and game argument assembly with placeholder substitution
- the launch plan shape and an exec strategy interface (reference
  child-process implementation)
- the JNI surface the Android host binds to (`libbasalt_engine.so`, built for
  x86_64 and arm64 with NDK 27)

A parity test proves the Rust plan matches the arguments the app actually
launched with on-device. The app calls the Rust engine over JNI for launch
policies (GC baseline, device-proportional memory default); the Kotlin
`:engine` module remains as the reference implementation and its tests still
run in Gradle builds.

What the engine deliberately does NOT reimplement, because those are compiled
artifacts from years of cross-compilation work rather than code that can be
re-derived from observation in a fork session:

- the bionic-built Java runtimes (upstream PojavLauncher-team OpenJDK builds)
- the in-process JVM exec native bridge and linker hooks (libpojavexec and
  friends)
- the patched LWJGL builds that bridge Android EGL/input into LWJGL
- the Compose UI shell (Kotlin; the Rust engine is a library, not an app)

The app binds its launch behavior to the engine through
`gcPolicyFromArgs`/`applyTo` (the same helpers the engine's tests enforce), so
the tested logic is the single source of truth for GC and memory policy. The
remaining upstream primitives stay credited artifacts under GPL-3.0/LGPL-3.0.

## Third-party assets added by this fork

- Geist and Geist Mono typefaces (Vercel), OFL 1.1, bundled in
  `ZalithLauncher/src/main/res/font/`.
- Phosphor Icons (phosphoricons.com), MIT license, converted to Android vector
  drawables in `ZalithLauncher/src/main/res/drawable/` (path data preserved,
  fill color per vector-drawable conventions).

## What this fork does not change

- Mojang's game files are downloaded at runtime from Mojang's own public
  endpoints (`piston-meta.mojang.com`, `piston-data.mojang.com`,
  `resources.download.minecraft.net`). No game code or assets are stored in
  this repository.
- The renderer and driver plugin ecosystem URLs still point at the upstream
  projects that publish those plugins (FCL-Team, ShirosakiMio, ZalithLauncher
  NativeLibPlugin). Those are functional resources, not branding.

## Identity

Basalt is its own product: its own name, icon, colors, and repository. It is
not affiliated with Mojang Studios or Microsoft, and it does not redistribute
any of their code or assets.
