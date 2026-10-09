//! Differential + unit tests against the real Mojang 26.3 manifest.

use basalt_engine::manifest::{Platform, VersionManifest};
use basalt_engine::plan::{AuthContext, EngineOptions, InstallLayout, MemoryOptions, PlanBuilder};
use basalt_engine::policy::{default_ram_allocation, gc_policy_from_args, GcPolicy};

fn manifest() -> VersionManifest {
    let raw = include_str!("fixtures/26.3.json");
    serde_json::from_str(raw).expect("26.3.json fixture must parse")
}

fn layout() -> InstallLayout {
    let root = std::env::temp_dir().join("basalt-engine-rs-test");
    InstallLayout {
        client_jar: root.join("versions/26.3/26.3.jar"),
        libraries_dir: root.join("libraries"),
        natives_dir: root.join("natives"),
        game_dir: root.join(".minecraft"),
        assets_dir: root.join("assets"),
        asset_index_id: "34".into(),
        runtime_home: root.join("runtime"),
    }
}

fn auth() -> AuthContext {
    AuthContext {
        player_name: "basalt_test".into(),
        uuid: "11112222333344445555666677778888".into(),
        ..AuthContext::default()
    }
}

fn default_options() -> EngineOptions {
    EngineOptions {
        memory: Some(MemoryOptions {
            heap_mb: 1637,
            min_mb: 512,
            max_mb: 8192,
        }),
        gc: Some("tunedG1".into()),
        extra_jvm_args: vec![],
        extra_game_args: vec![],
        active_processors: Some(8),
        strip_always_pre_touch: true,
    }
}

fn build(
    manifest: &VersionManifest,
    layout: &InstallLayout,
    auth: &AuthContext,
    options: &EngineOptions,
) -> basalt_engine::plan::LaunchPlan {
    PlanBuilder {
        manifest,
        platform: Platform::LinuxX8664,
        layout,
        features: Default::default(),
        options: options.clone(),
        auth: auth.clone(),
        version_name: "26.3".into(),
        version_type: "release".into(),
        quick_play: None,
        log_config_path: None,
    }
    .build()
}

#[test]
fn real_manifest_parses_with_expected_shape() {
    let m = manifest();
    assert_eq!(m.id, "26.3");
    assert_eq!(m.java_version.as_ref().unwrap().major_version, 25);
    assert_eq!(m.libraries.len(), 114);
    assert_eq!(m.main_class, "net.minecraft.client.main.Main");
}

#[test]
fn classpath_on_linux_x86_64_is_client_plus_allowed_libraries() {
    let m = manifest();
    let plan = build(&m, &layout(), &auth(), &default_options());
    let context = basalt_engine::rules::RuleContext {
        platform: Platform::LinuxX8664,
        features: Default::default(),
    };
    let allowed: Vec<_> = m
        .libraries
        .iter()
        .filter(|l| context.is_allowed(l))
        .filter(|l| {
            l.downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .map(|a| a.path.as_deref().map(|p| !p.trim().is_empty()).unwrap_or(false))
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(plan.classpath.len(), 1 + allowed.len());
    assert!(plan.classpath.first().unwrap().ends_with("26.3.jar"));
    let unique: std::collections::HashSet<_> = plan.classpath.iter().collect();
    assert_eq!(unique.len(), plan.classpath.len());
}

#[test]
fn linux_natives_on_classpath_and_others_absent() {
    let m = manifest();
    let plan = build(&m, &layout(), &auth(), &default_options());
    assert!(plan
        .classpath
        .iter()
        .any(|p| p.contains("lwjgl") && p.contains("natives-linux")));
    assert!(!plan.classpath.iter().any(|p| p.contains("natives-macos")));
    assert!(!plan.classpath.iter().any(|p| p.contains("natives-windows")));
}

#[test]
fn memory_policy_replaces_manifest_heap_flags() {
    let m = manifest();
    let plan = build(&m, &layout(), &auth(), &default_options());
    assert!(!plan.jvm_args.contains(&"-Xms2G".to_string()));
    assert!(!plan.jvm_args.contains(&"-Xmx4G".to_string()));
    assert!(plan.jvm_args.contains(&"-Xms1637M".to_string()));
    assert!(plan.jvm_args.contains(&"-Xmx1637M".to_string()));
}

#[test]
fn tuned_g1_strips_manifest_collector_selection() {
    let m = manifest();
    let plan = build(&m, &layout(), &auth(), &default_options());
    assert!(!plan.jvm_args.contains(&"-XX:+UseZGC".to_string()));
    assert!(plan.jvm_args.contains(&"-XX:+UseG1GC".to_string()));
    assert!(plan.jvm_args.contains(&"-XX:G1NewSizePercent=20".to_string()));
    assert!(plan.jvm_args.contains(&"-XX:G1ReservePercent=20".to_string()));
    assert!(plan.jvm_args.contains(&"-XX:MaxGCPauseMillis=40".to_string()));
    assert!(plan.jvm_args.contains(&"-XX:G1HeapRegionSize=16M".to_string()));
    assert!(plan.jvm_args.contains(&"-XX:+DisableExplicitGC".to_string()));
}

#[test]
fn object_layout_flags_survive_gc_override() {
    let m = manifest();
    let plan = build(&m, &layout(), &auth(), &default_options());
    assert!(plan.jvm_args.contains(&"-XX:+UseCompactObjectHeaders".to_string()));
}

#[test]
fn always_pre_touch_dropped_for_phone_heaps_kept_for_large() {
    let m = manifest();
    let l = layout();
    let a = auth();

    let small_opts = default_options();
    let small = build(&m, &l, &a, &small_opts);
    assert!(!small.jvm_args.contains(&"-XX:+AlwaysPreTouch".to_string()));

    let large_opts = EngineOptions {
        memory: Some(MemoryOptions {
            heap_mb: 6144,
            min_mb: 512,
            max_mb: 8192,
        }),
        ..default_options()
    };
    let large = build(&m, &l, &a, &large_opts);
    assert!(large.jvm_args.contains(&"-XX:+AlwaysPreTouch".to_string()));
}

#[test]
fn custom_gc_passes_user_flags_through() {
    let args = vec![
        "-XX:+UseZGC".to_string(),
        "-XX:ZCollectionInterval=5".to_string(),
        "-Xmx2G".to_string(),
    ];
    let policy = gc_policy_from_args(&args);
    assert!(matches!(policy, GcPolicy::Custom(_)));
    let mut out = args.clone();
    policy.apply_to(&mut out);
    // User flags untouched by the policy application.
    assert!(out.contains(&"-XX:+UseZGC".to_string()));
    assert!(!out.contains(&"-XX:+UseG1GC".to_string()));
}

#[test]
fn active_processor_count_pinned() {
    let m = manifest();
    let plan = build(&m, &layout(), &auth(), &default_options());
    assert!(plan.jvm_args.contains(&"-XX:ActiveProcessorCount=8".to_string()));
}

#[test]
fn game_args_have_no_unresolved_placeholders() {
    let m = manifest();
    let plan = build(&m, &layout(), &auth(), &default_options());
    assert!(plan.game_args.contains(&"basalt_test".to_string()));
    assert!(plan
        .game_args
        .contains(&"11112222333344445555666677778888".to_string()));
    assert!(!plan.game_args.iter().any(|a| a.contains("${")));
}

#[test]
fn plan_command_list_is_well_formed() {
    let m = manifest();
    let plan = build(&m, &layout(), &auth(), &default_options());
    let command = plan.to_command_list("/runtime/bin/java");
    assert_eq!(command.first().unwrap(), "/runtime/bin/java");
    assert!(command.contains(plan.wrapper_class.as_ref().unwrap()));
    assert!(command.iter().any(|s| s == "net.minecraft.client.main.Main"));
}

#[test]
fn default_ram_allocation_matches_the_kotlin_engine() {
    // 8 GB device: max 7168, target 4505 -> 4505
    assert_eq!(default_ram_allocation(8192.0, false), 4505);
    // The emulator's healthy reading: 2976 MB -> 1637
    assert_eq!(default_ram_allocation(2976.6, false), 1637);
    // A degraded sub-2 GB reading: the <2048 branch caps at min(1024, ram)
    assert_eq!(default_ram_allocation(1736.0, false), 1024);
    // 32-bit path: capped at min(1024, ram)
    assert_eq!(default_ram_allocation(500.0, true), 500);
    // Sub-2 GB 64-bit: max = min(1024, ram)
    assert_eq!(default_ram_allocation(1000.0, false), 1000);
}

#[test]
fn platform_mapping() {
    assert_eq!(Platform::from("Linux", "aarch64"), Some(Platform::LinuxArm64));
    assert_eq!(Platform::from("Linux", "amd64"), Some(Platform::LinuxX8664));
    assert_eq!(Platform::from("windows", "amd64"), None);
    assert_eq!(Platform::from("linux", "i386"), None);
}
