# Changelog

All notable changes to this fork are recorded here. Upstream history lives in
the git lineage of the fork point.

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
