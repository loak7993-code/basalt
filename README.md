# Basalt

A Minecraft: Java Edition launcher for Android, focused on offline (offline-mode)
accounts, modern versions, and honest performance tuning on phone-class hardware.

Basalt is a fork of [ZalithLauncher 2](https://github.com/ZalithLauncher/ZalithLauncher2)
(GPL-3.0) by MovTery and contributors. The runtime core (the Android JVM port,
LWJGL patches, GL renderers, input bridge) is upstream GPL code, credited and
unchanged in lineage. The product layer in this fork is rebuilt: locked dark
theme with a single emerald accent, Geist typography, Phosphor bold icon set,
offline-account-first flows, device-proportional memory defaults, a tuned G1
baseline for the game JVM, and a fresh brand identity.

This launcher is not affiliated with, endorsed by, or connected to Mojang
Studios or Microsoft. Java Edition game files are downloaded at runtime from
Mojang's own public endpoints. Playing multiplayer on online-mode servers
requires a legitimate Minecraft account; offline accounts exist for offline
play and offline-mode servers.

## Features

- Java Edition from 1.7 up to the current drop, MC 26.3 included (Java 25 runtime bundled)
- Offline accounts first: the login menu opens on offline login; Microsoft and
  authlib-injector accounts still work
- Version install from the official Mojang manifest, with parallel download and
  SHA1 verification
- Renderer options (gl4es, ANGLE, Zink, VirGL) from the upstream plugin ecosystem
- Virtual mouse, joystick, gyroscope, gamepad support; control layout editor
- Terracotta multiplayer rooms, mod loader installs (Forge/Fabric/NeoForge/OptiFine/Quilt)
- Locked dark interface: near-black canvas, single emerald accent, Geist Sans/Mono,
  Phosphor bold icons

## Screenshots

Real captures from the verification run on an Android 15 emulator (2400x1080):

| | |
|---|---|
| ![Home with 26.3 and an offline account](docs/screenshots/home-263.png) | ![Download list with 26.3 on top](docs/screenshots/download-263.png) |
| ![Offline account created and selected](docs/screenshots/offline-account.png) | ![Home empty state with guide](docs/screenshots/home-empty.png) |

## Quick start

Build:

```bash
git clone https://github.com/Leonxlnx/Basalt.git
cd Basalt
./gradlew :ZalithLauncher:assembleDebug
```

The debug APK lands in `ZalithLauncher/build/outputs/apk/debug/`. Install it on
an Android 8.0+ (API 26) device and allow install-from-unknown-sources.

First run on the device:

1. The launcher unpacks its bundled Java runtimes (one-time).
2. Add an offline account: Accounts, then pick Offline, and type a name.
3. Download: Game, pick 26.3 (top of the Releases list), install.
4. Home: pick the version card, Launch.

## Configuration

No build-time configuration is required. Optional properties live in
`ZalithLauncher/gradle.properties`:

| Property | Purpose | Default |
|---|---|---|
| `launcher_name` | App label + data-dir identity | `Basalt` |
| `launcher_app_name` | Full display name | `Basalt` |
| `launcher_short_name` | Short name used in user agents | `Basalt` |
| `url_home` | Fork home page (update/about links) | `https://github.com/Leonxlnx/Basalt` |
| `launcher_version_code` | Android versionCode | `10000` |
| `launcher_version_name` | Android versionName | `1.0.0` |
| `basalt_store_password` | Release keystore password | `basalt-2026-release` |
| `basalt_key_password` | Release key password | `basalt-2026-release` |
| `oauth_client_id` | Microsoft OAuth client id for MS login | unset (offline login needs none) |
| `curseforge_api_key` | CurseForge API key for mod downloads | unset |

In-app settings that matter for performance live under Settings: memory
allocation (defaults to ~55% of device RAM, minimum 1536 MB where the device
allows), renderer selection, and custom JVM args. When no custom GC is set, the
launcher injects a tuned G1 baseline (`G1NewSizePercent=20`,
`G1ReservePercent=20`, `MaxGCPauseMillis=40`, `G1HeapRegionSize=16M`).

## Build, test, deploy

```bash
./gradlew :ZalithLauncher:assembleDebug            # universal debug APK
./gradlew :ZalithLauncher:assembleDebug -Darch=x86_64   # single-ABI debug APK
./gradlew :ZalithLauncher:assembleRelease          # release APK, signed with basalt.jks
./gradlew :ZalithLauncher:assembleRelease -Darch=arm64  # arm64-only release APK
```

Builds need JDK 21 (the toolchain compiles with it) and an Android SDK with
platform 37, build-tools 37, NDK 25.2.9519653, and accepted licenses.

## Architecture

```
Basalt/
  ZalithLauncher/        the app: Compose UI, settings, accounts, downloads, launch pipeline
    src/main/java/com/movtery/zalithlauncher/
      game/launch/       JVM launch pipeline, arg assembly, GC baseline
      game/download/     version/asset download engine (official Mojang endpoints)
      ui/screens/        Compose screens (home, download, accounts, settings, game)
      ui/theme/          Basalt palette, Geist typography
      components/jre/    bundled runtime unpacking (jre-8/17/21/25)
      jni/               native bridge (EGL/input/linker hooks), upstream
    src/main/assets/runtimes/  bionic JRE builds per Java version
  LWJGL/                 upstream patched LWJGL (3.3.3, 3.4.1)
  Terracotta/            multiplayer room client
  Guide/ CardGrid/ ColorPicker/ InputMap/ LayerController/   UI modules
```

## Known limitations

- Verified here only on an x86_64 emulator (API 35): install, theme, screens,
  navigation, settings. Real-device launch of MC 26.3 was not tested in this
  environment; on-device rendering depends on the device GPU and chosen renderer.
- Microsoft login is disabled by default (no OAuth client id configured).
  Configure `oauth_client_id` with your own Azure app id to enable it.
- The in-app upgrade check points at the fork's info repo; while that repo has
  no release metadata, the check quietly reports nothing.
- Upstream seasonal festival effects are off by default.
- Modloader installers for versions newer than 26.3 depend on the upstream
  modloader metadata sources catching up.
