//! Parity: the Rust plan must match the args the app actually launched with
//! on-device (captured from the persisted game log of the 1.1.0/1.2.0 runs).

use basalt_engine::manifest::{Platform, VersionManifest};
use basalt_engine::plan::{AuthContext, EngineOptions, InstallLayout, MemoryOptions, PlanBuilder};

fn app_options() -> EngineOptions {
    EngineOptions {
        memory: Some(MemoryOptions {
            heap_mb: 1637,
            min_mb: 512,
            max_mb: 8192,
        }),
        gc: Some("tunedG1".into()),
        extra_jvm_args: vec![],
        extra_game_args: vec![],
        active_processors: Some(4),
        strip_always_pre_touch: true,
        ..Default::default()
    }
}

#[test]
fn rust_plan_matches_the_captured_device_launch_args() {
    let raw = include_str!("fixtures/26.3.json");
    let manifest: VersionManifest = serde_json::from_str(raw).unwrap();
    let layout = InstallLayout {
        client_jar: "/tmp/26.3.jar".into(),
        libraries_dir: "/tmp/libraries".into(),
        natives_dir: "/tmp/natives".into(),
        game_dir: "/storage/emulated/0/Android/data/dev.leonxlnx.basalt.debug/files/.minecraft"
            .into(),
        assets_dir: "/tmp/assets".into(),
        asset_index_id: "34".into(),
        runtime_home: "/tmp/runtime".into(),
    };
    let plan = PlanBuilder {
        manifest: &manifest,
        platform: Platform::LinuxX8664,
        layout: &layout,
        features: Default::default(),
        options: app_options(),
        auth: AuthContext {
            player_name: "basalt_test".into(),
            ..AuthContext::default()
        },
        version_name: "26.3".into(),
        version_type: "release".into(),
        quick_play: None,
        log_config_path: None,
    }
    .build();

    // Captured from versions/26.3/Basalt/latest_game.log on the emulator.
    let captured: Vec<String> = vec![
        "-Xms1637M".into(),
        "-Xmx1637M".into(),
        "-XX:ActiveProcessorCount=4".into(),
        "-XX:+UnlockExperimentalVMOptions".into(),
        "-XX:+UseG1GC".into(),
        "-XX:G1NewSizePercent=20".into(),
        "-XX:G1ReservePercent=20".into(),
        "-XX:G1HeapRegionSize=16M".into(),
        "-XX:+DisableExplicitGC".into(),
    ];
    for arg in &captured {
        assert!(
            plan.jvm_args.contains(arg),
            "missing from the Rust plan: {}",
            arg
        );
    }
    assert!(!plan.jvm_args.contains(&"-XX:+UseZGC".to_string()));
    assert!(!plan.jvm_args.contains(&"-Xms2G".to_string()));
}
