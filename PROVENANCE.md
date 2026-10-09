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
