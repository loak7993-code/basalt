//! Crash classification tests.
//!
//! The renderer case runs against a log captured from a real Basalt launch of
//! MC 26.3 (Linux, x86_64, llvmpipe), not an invented string. The rest cover
//! the signatures the JVM, Forge and the download engine print.

use basalt_engine::diagnose::{self, CrashKind, Fix, LaunchContext, Renderer};

fn lines(raw: &str) -> Vec<String> {
    raw.lines().map(str::to_string).collect()
}

/// The tail of a real 26.3 launch that died in backend creation on a machine
/// with no usable GL driver. Truncated after the failure.
const REAL_RENDERER_CRASH: &str = r#"[18:03:37] [Render thread/INFO]: Environment: Environment[discoveryUrl=https://discovery.minecraftservices.com/minecraft/client, name=PROD]
[18:03:37] [Render thread/INFO]: Setting user: basalt_linux
[18:03:37] [Render thread/INFO]: Backend library: LWJGL version 3.4.3+4
[18:03:37] [Render thread/ERROR]: Failed to create backend OpenGL
com.mojang.renderpearl.api.device.BackendCreationException: glGetError mismatch
	at com.mojang.renderpearl.backend.opengl.GlBackend.loadLibrary(GlBackend.java:40)
	at net.minecraft.client.Minecraft.<init>(Minecraft.java:530)
	at net.minecraft.client.main.Main.main(Main.java:278)
"#;

#[test]
fn real_26_3_backend_failure_is_renderer_init() {
    let ctx = LaunchContext {
        renderer: Some(Renderer::Gl4es),
        ..Default::default()
    };
    let d = diagnose::diagnose(&lines(REAL_RENDERER_CRASH), 1, &ctx);
    assert_eq!(d.kind, CrashKind::RendererInit);
    assert_eq!(d.exit_code, 1);
    assert_eq!(
        d.fix,
        Fix::SetRenderer {
            renderer: Renderer::Zink
        }
    );
    assert!(
        d.evidence
            .iter()
            .any(|l| l.contains("Failed to create backend OpenGL")),
        "evidence must quote the log, got {:?}",
        d.evidence
    );
}

#[test]
fn a_healthy_log_is_not_diagnosed() {
    let healthy = lines(
        "[18:03:37] [Render thread/INFO]: Setting user: basalt_test\n\
         [18:03:38] [Render thread/INFO]: Using graphics backend Vulkan\n",
    );
    let d = diagnose::diagnose(&healthy, 0, &LaunchContext::default());
    assert_eq!(d.kind, CrashKind::Unknown);
    assert!(matches!(d.fix, Fix::None { .. }));
}

#[test]
fn heap_exhaustion_grows_the_heap() {
    let ctx = LaunchContext {
        heap_mb: Some(1024),
        device_ram_mb: Some(8192.0),
        ..Default::default()
    };
    let d = diagnose::diagnose(
        &lines("java.lang.OutOfMemoryError: Java heap space\n"),
        1,
        &ctx,
    );
    assert_eq!(d.kind, CrashKind::HeapExhausted);
    match d.fix {
        Fix::SetHeap {
            heap_mb: Some(mb), ..
        } => assert_eq!(mb, 1536, "1024 grows by half"),
        other => panic!("expected a heap fix, got {other:?}"),
    }
}

#[test]
fn heap_fix_stays_inside_the_device_budget() {
    // 3 GB device, 2 GB heap already: growing by half would ask for 3 GB,
    // which is the whole device. The fix must stop at 75% of RAM.
    let ctx = LaunchContext {
        heap_mb: Some(2048),
        device_ram_mb: Some(3072.0),
        ..Default::default()
    };
    let d = diagnose::diagnose(
        &lines("java.lang.OutOfMemoryError: Java heap space\n"),
        1,
        &ctx,
    );
    match d.fix {
        Fix::SetHeap {
            heap_mb: Some(mb), ..
        } => {
            assert_eq!(mb, 2304, "the 75% device budget wins over the 50% growth")
        }
        other => panic!("expected a heap fix, got {other:?}"),
    }
}

#[test]
fn heap_fix_declines_when_the_budget_is_spent() {
    let ctx = LaunchContext {
        heap_mb: Some(6144),
        device_ram_mb: Some(8192.0),
        ..Default::default()
    };
    let d = diagnose::diagnose(&lines("OutOfMemoryError: Metaspace\n"), 1, &ctx);
    assert!(
        matches!(d.fix, Fix::None { .. }),
        "must not ask for more heap than the device has, got {:?}",
        d.fix
    );
}

#[test]
fn renderer_fallback_walks_the_other_options() {
    for (current, expected) in [
        (Renderer::Gl4es, Renderer::Zink),
        (Renderer::Zink, Renderer::VirGl),
        (Renderer::VirGl, Renderer::Gl4es),
    ] {
        let ctx = LaunchContext {
            renderer: Some(current),
            ..Default::default()
        };
        let d = diagnose::diagnose(&lines("Failed to create backend OpenGL\n"), 1, &ctx);
        assert_eq!(d.fix, Fix::SetRenderer { renderer: expected });
    }
}

#[test]
fn desktop_renderer_failures_are_reported_not_guessed() {
    // No renderer setting on desktop: the platform picks it, so the engine
    // must not pretend it can switch.
    let d = diagnose::diagnose(
        &lines("Failed to create backend OpenGL\n"),
        1,
        &LaunchContext::default(),
    );
    assert_eq!(d.kind, CrashKind::RendererInit);
    assert!(matches!(d.fix, Fix::None { .. }));
}

#[test]
fn newer_java_is_named_from_the_class_file_error() {
    let ctx = LaunchContext {
        java_major: Some(17),
        ..Default::default()
    };
    let d = diagnose::diagnose(
        &lines("java.lang.UnsupportedClassVersionError: class file version 69.0, this runtime only understands up to 61.0\n"),
        1,
        &ctx,
    );
    assert_eq!(d.kind, CrashKind::UnsupportedJava);
    assert_eq!(d.fix, Fix::UseJava { major: 21 });
}

#[test]
fn a_new_enough_java_is_left_alone() {
    let ctx = LaunchContext {
        java_major: Some(25),
        ..Default::default()
    };
    let d = diagnose::diagnose(&lines("UnsupportedClassVersionError\n"), 1, &ctx);
    assert!(matches!(d.fix, Fix::None { .. }));
}

#[test]
fn checksum_failure_names_the_file_to_refetch() {
    let d = diagnose::diagnose(
        &lines("[Download-2/ERROR]: hash mismatch for \"/versions/26.3/26.3.jar\"\n"),
        1,
        &LaunchContext::default(),
    );
    assert_eq!(d.kind, CrashKind::CorruptAsset);
    assert_eq!(
        d.fix,
        Fix::RefetchFiles {
            paths: vec!["/versions/26.3/26.3.jar".to_string()]
        }
    );
}

#[test]
fn a_checksum_failure_without_a_path_is_not_acted_on() {
    let d = diagnose::diagnose(
        &lines("Checksum does not match\n"),
        1,
        &LaunchContext::default(),
    );
    assert_eq!(d.kind, CrashKind::CorruptAsset);
    assert!(matches!(d.fix, Fix::None { .. }));
}

#[test]
fn jvm_argument_errors_are_never_retried() {
    // Relaunching the same bad arguments would fail identically forever.
    let d = diagnose::diagnose(
        &lines("Unrecognized VM option 'MadeUpFlag'\n"),
        1,
        &LaunchContext {
            heap_mb: Some(1024),
            device_ram_mb: Some(8192.0),
            ..Default::default()
        },
    );
    assert_eq!(d.kind, CrashKind::JvmStartFailure);
    assert!(matches!(d.fix, Fix::None { .. }));
}

#[test]
fn dependency_resolution_beats_mixin_failure() {
    // A pack that fails to resolve often prints mixin noise first; the root
    // cause is the unresolved dependency.
    let log = lines(
        "Mod file minecraft.jar has an unsupported dependency\n\
         Missing mandatory dependencies, version: fabric-api\n\
         Mixin apply failed: mixin.class\n",
    );
    let d = diagnose::diagnose(&log, 1, &LaunchContext::default());
    assert_eq!(d.kind, CrashKind::DependencyResolution);
}

#[test]
fn a_mod_the_log_names_is_disabled_by_rename() {
    let dir = std::env::temp_dir().join("basalt-diagnose-mods");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mod_jar = dir.join("brokenmod-1.2.3.jar");
    std::fs::write(&mod_jar, b"not a real jar").unwrap();

    let ctx = LaunchContext {
        mods_dir: Some(dir.clone()),
        ..Default::default()
    };
    let d = diagnose::diagnose(&lines("Failed to load mod brokenmod-1.2.3.jar\n"), 1, &ctx);
    assert_eq!(d.kind, CrashKind::DependencyResolution);
    match &d.fix {
        Fix::DisableMods { files } => {
            assert_eq!(files, &vec![mod_jar.clone()]);
            // The rename must be reversible, and must actually move the file.
            let renamed = diagnose::disable_mods(files).unwrap();
            assert!(renamed[0]
                .to_string_lossy()
                .ends_with(".jar.basalt-disabled"));
            assert!(!mod_jar.exists());
            let restored = diagnose::enable_mods(&renamed).unwrap();
            assert_eq!(restored, vec![mod_jar.clone()]);
            assert!(mod_jar.exists(), "the mod must come back byte for byte");
            assert_eq!(
                std::fs::read(&mod_jar).unwrap(),
                b"not a real jar",
                "restoring must not touch the contents"
            );
        }
        other => panic!("expected the named mod to be disabled, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_mod_that_is_not_on_disk_is_not_disabled() {
    let dir = std::env::temp_dir().join("basalt-diagnose-empty");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let ctx = LaunchContext {
        mods_dir: Some(dir.clone()),
        ..Default::default()
    };
    let d = diagnose::diagnose(&lines("Failed to load mod ghost-9.9.9.jar\n"), 1, &ctx);
    assert!(matches!(d.fix, Fix::None { .. }));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_tail_is_always_carried_for_the_report() {
    let long: Vec<String> = (0..200).map(|i| format!("line {i}")).collect();
    let d = diagnose::diagnose(&long, 1, &LaunchContext::default());
    assert_eq!(d.tail.len(), 24);
    assert_eq!(d.tail.last().unwrap(), "line 199");
}

#[test]
fn renderer_names_round_trip() {
    for (text, expected) in [
        ("gl4es", Some(Renderer::Gl4es)),
        ("VirGL", Some(Renderer::VirGl)),
        ("zink", Some(Renderer::Zink)),
        ("mesa", None),
    ] {
        assert_eq!(Renderer::parse(text), expected);
    }
}
