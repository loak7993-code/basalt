//! Supervised launch loop tests.
//!
//! The loop itself runs against a scripted exec, and one end-to-end case runs
//! a real JVM that really does run out of heap, so the classifier is proved
//! against a genuine crash rather than only against fixtures.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use basalt_engine::exec::ExecStrategy;
use basalt_engine::plan::LaunchPlan;
use basalt_engine::supervisor::{
    has_fatal_marker, Attempt, HeapTweaker, LaunchReport, Limits, PlanTweaker, Supervisor,
};

fn plan() -> LaunchPlan {
    LaunchPlan {
        jvm_args: vec!["-Xmx512M".into(), "-XX:+UseG1GC".into()],
        game_args: vec!["--username".into(), "tester".into()],
        main_class: "net.minecraft.client.main.Main".into(),
        classpath: vec!["/versions/26.3/26.3.jar".into()],
        classpath_string: "/versions/26.3/26.3.jar".into(),
        wrapper_class: None,
        required_java_major: Some(25),
        log_config_sha1: None,
    }
}

/// Exec that replays a scripted sequence: each call prints its lines and
/// returns its exit code. Records the plan it was handed so tests can assert
/// what the loop changed.
struct ScriptedExec {
    steps: Vec<(Vec<String>, i32)>,
    seen: Arc<Mutex<Vec<Vec<String>>>>,
    calls: AtomicUsize,
}

impl ExecStrategy for ScriptedExec {
    fn exec(&self, plan: &LaunchPlan, _java: &str, sink: &mut dyn FnMut(&str)) -> i32 {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().unwrap().push(plan.jvm_args.clone());
        let (lines, code) = self
            .steps
            .get(index)
            .cloned()
            .unwrap_or_else(|| (vec!["exhausted the script".into()], -1));
        for line in lines {
            sink(&line);
        }
        code
    }
}

fn collector() -> Arc<Mutex<Vec<String>>> {
    Arc::new(Mutex::new(Vec::new()))
}

/// Records the JVM args each attempt was launched with.
fn attempt_recorder() -> Arc<Mutex<Vec<Vec<String>>>> {
    Arc::new(Mutex::new(Vec::new()))
}

fn sink_for(seen: Arc<Mutex<Vec<String>>>) -> basalt_engine::supervisor::LineSink {
    Arc::new(move |line: &str| {
        seen.lock().unwrap().push(line.to_string());
    })
}

#[test]
fn a_healthy_launch_runs_once() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let exec = ScriptedExec {
        steps: vec![(vec!["[Render thread/INFO]: Setting user: tester".into()], 0)],
        seen: attempt_recorder(),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext::default(),
    );
    let report = supervisor.run(&plan(), "/usr/bin/java", sink_for(seen));

    assert_eq!(report.exit_code, 0);
    assert_eq!(report.attempts.len(), 1);
    assert!(report.succeeded());
    assert!(!report.recovered());
    assert!(report.diagnosis().is_none());
}

#[test]
fn a_heap_crash_relaunches_once_with_a_bigger_heap() {
    let output = collector();
    let plans = attempt_recorder();
    let exec = ScriptedExec {
        steps: vec![
            (
                vec!["java.lang.OutOfMemoryError: Java heap space".into()],
                1,
            ),
            (vec!["[Render thread/INFO]: Loaded".into()], 0),
        ],
        seen: Arc::clone(&plans),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext {
            heap_mb: Some(512),
            device_ram_mb: Some(4096.0),
            ..Default::default()
        },
    );
    let report = supervisor.run(&plan(), "/usr/bin/java", sink_for(Arc::clone(&output)));

    assert!(report.recovered(), "the second attempt must succeed");
    assert_eq!(report.attempts.len(), 2);
    assert_eq!(
        report.attempts[0].kind(),
        Some(basalt_engine::CrashKind::HeapExhausted)
    );

    let attempts = plans.lock().unwrap();
    assert_eq!(attempts[0], vec!["-Xmx512M", "-XX:+UseG1GC"]);
    assert_eq!(attempts[1][0], "-Xmx768M", "512 grew by half to 768");
    assert_eq!(
        attempts[1].len(),
        2,
        "the heap arg is replaced, not appended"
    );

    let printed = output.lock().unwrap();
    assert!(
        printed
            .iter()
            .any(|l| l.contains("Relaunching with a 768 MB heap")),
        "the player is told what changed: {:?}",
        printed
    );
}

#[test]
fn a_heap_crash_with_no_heap_set_gets_one() {
    let plans = attempt_recorder();
    let mut bare = plan();
    bare.jvm_args = vec!["-XX:+UseG1GC".into()];
    let exec = ScriptedExec {
        steps: vec![
            (
                vec!["java.lang.OutOfMemoryError: Java heap space".into()],
                1,
            ),
            (vec!["ok".into()], 0),
        ],
        seen: Arc::clone(&plans),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext::default(),
    );
    supervisor.run(&bare, "/usr/bin/java", sink_for(collector()));

    let attempts = plans.lock().unwrap();
    assert!(
        attempts[1].iter().any(|a| a.starts_with("-Xmx")),
        "a heap must be added when the plan had none: {:?}",
        attempts[1]
    );
}

#[test]
fn the_same_fix_is_never_applied_twice() {
    // A log that always says OOM: without a dedupe the loop would raise the
    // heap until the attempt budget runs out.
    let plans = attempt_recorder();
    let exec = ScriptedExec {
        steps: vec![
            (
                vec!["java.lang.OutOfMemoryError: Java heap space".into()],
                1,
            ),
            (
                vec!["java.lang.OutOfMemoryError: Java heap space".into()],
                1,
            ),
        ],
        seen: Arc::clone(&plans),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext {
            heap_mb: Some(512),
            device_ram_mb: Some(4096.0),
            ..Default::default()
        },
    );
    let report = supervisor.run(&plan(), "/usr/bin/java", sink_for(collector()));

    let attempts = plans.lock().unwrap();
    assert_eq!(attempts.len(), 2, "second crash, same fix, loop stops");
    assert_eq!(attempts[1][0], "-Xmx768M");
    assert_eq!(report.exit_code, 1);
}

#[test]
fn a_budget_of_one_never_relaunches() {
    // The crash is fixable, but the caller allowed exactly one launch.
    let plans = attempt_recorder();
    let exec = ScriptedExec {
        steps: vec![
            (
                vec!["java.lang.OutOfMemoryError: Java heap space".into()],
                1,
            ),
            (vec!["ok".into()], 0),
        ],
        seen: Arc::clone(&plans),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext {
            heap_mb: Some(512),
            device_ram_mb: Some(4096.0),
            ..Default::default()
        },
    )
    .with_limits(Limits {
        max_attempts: 1,
        heap_ceiling_mb: 8192,
    });
    let report = supervisor.run(&plan(), "/usr/bin/java", sink_for(collector()));
    assert_eq!(report.attempts.len(), 1);
    assert_eq!(plans.lock().unwrap().len(), 1);
    assert_eq!(
        report.attempts[0].kind(),
        Some(basalt_engine::CrashKind::HeapExhausted),
        "the crash is still reported even when it was not fixed"
    );
}

#[test]
fn a_repeating_crash_stays_inside_the_budget() {
    let plans = attempt_recorder();
    let steps: Vec<(Vec<String>, i32)> = (0..12)
        .map(|_| {
            (
                vec!["java.lang.OutOfMemoryError: Java heap space".into()],
                1,
            )
        })
        .collect();
    let exec = ScriptedExec {
        steps,
        seen: Arc::clone(&plans),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext {
            heap_mb: Some(512),
            device_ram_mb: Some(4096.0),
            ..Default::default()
        },
    )
    .with_limits(Limits {
        max_attempts: 10,
        heap_ceiling_mb: 8192,
    });
    let report = supervisor.run(&plan(), "/usr/bin/java", sink_for(collector()));
    assert!(
        report.attempts.len() <= 10,
        "the budget is a hard ceiling, got {}",
        report.attempts.len()
    );
    assert_eq!(report.attempts.len(), 2, "the dedupe stops it first");
}

#[test]
fn the_heap_ceiling_overrides_the_device_budget() {
    let plans = attempt_recorder();
    let exec = ScriptedExec {
        steps: vec![
            (
                vec!["java.lang.OutOfMemoryError: Java heap space".into()],
                1,
            ),
            (vec!["ok".into()], 0),
        ],
        seen: Arc::clone(&plans),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext {
            heap_mb: Some(8000),
            device_ram_mb: Some(64_000.0),
            ..Default::default()
        },
    )
    .with_limits(Limits {
        max_attempts: 3,
        heap_ceiling_mb: 8192,
    });
    supervisor.run(&plan(), "/usr/bin/java", sink_for(collector()));
    let attempts = plans.lock().unwrap();
    assert_eq!(attempts[1][0], "-Xmx8192M", "clamped to the limit");
}

#[test]
fn a_fix_the_host_cannot_do_ends_the_loop() {
    // Renderer fallback needs a host that owns the setting. A desktop host has
    // none, so the loop must report rather than pretend.
    let plans = attempt_recorder();
    let exec = ScriptedExec {
        steps: vec![
            (vec!["Failed to create backend OpenGL".into()], 1),
            (vec!["ok".into()], 0),
        ],
        seen: Arc::clone(&plans),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext {
            renderer: Some(basalt_engine::Renderer::Gl4es),
            ..Default::default()
        },
    );
    let report = supervisor.run(&plan(), "/usr/bin/java", sink_for(collector()));
    assert_eq!(plans.lock().unwrap().len(), 1, "no pointless relaunch");
    assert_eq!(
        report.attempts[0].kind(),
        Some(basalt_engine::CrashKind::RendererInit)
    );
}

#[test]
fn a_host_with_a_renderer_setting_does_relaunch() {
    struct AndroidLike;
    impl PlanTweaker for AndroidLike {
        fn set_heap(&self, plan: &mut LaunchPlan, heap_mb: i64) -> bool {
            HeapTweaker.set_heap(plan, heap_mb)
        }
        fn set_renderer(&self, plan: &mut LaunchPlan, renderer: basalt_engine::Renderer) -> bool {
            plan.game_args
                .push(format!("--renderer={}", renderer.as_str()));
            true
        }
    }

    let plans = attempt_recorder();
    let exec = ScriptedExec {
        steps: vec![
            (vec!["Failed to create backend OpenGL".into()], 1),
            (vec!["ok".into()], 0),
        ],
        seen: attempt_recorder(),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        AndroidLike,
        basalt_engine::diagnose::LaunchContext {
            renderer: Some(basalt_engine::Renderer::Gl4es),
            ..Default::default()
        },
    );
    let report = supervisor.run(&plan(), "/usr/bin/java", sink_for(collector()));
    assert!(report.recovered());
    let _ = plans;
}

#[test]
fn a_fatal_error_with_exit_zero_is_still_a_failure() {
    let exec = ScriptedExec {
        steps: vec![(
            vec![
                "[Server thread/ERROR]: Fatal crash!".into(),
                "Exception in thread \"main\" java.lang.NullPointerException".into(),
            ],
            0,
        )],
        seen: attempt_recorder(),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext::default(),
    );
    let report = supervisor.run(&plan(), "/usr/bin/java", sink_for(collector()));
    assert_eq!(report.attempts.len(), 1);
    assert!(
        report.attempts[0].diagnosis.is_some(),
        "exit 0 is not a pass"
    );
    assert!(has_fatal_marker(&[
        "Fatal crash!".to_string(),
        "Exception in thread \"main\"".to_string()
    ]));
}

#[test]
fn every_line_reaches_the_sink_on_every_attempt() {
    let output = collector();
    let exec = ScriptedExec {
        steps: vec![
            (
                vec![
                    "first attempt".into(),
                    "java.lang.OutOfMemoryError: Java heap space".into(),
                ],
                1,
            ),
            (vec!["second attempt".into()], 0),
        ],
        seen: attempt_recorder(),
        calls: AtomicUsize::new(0),
    };
    let supervisor = Supervisor::new(
        exec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext {
            heap_mb: Some(512),
            device_ram_mb: Some(4096.0),
            ..Default::default()
        },
    );
    supervisor.run(&plan(), "/usr/bin/java", sink_for(Arc::clone(&output)));
    let printed = output.lock().unwrap();
    assert!(printed.iter().any(|l| l == "first attempt"));
    assert!(printed.iter().any(|l| l == "second attempt"));
}

/// Runs a real JVM that genuinely exhausts a tiny heap, through the reference
/// exec strategy, and checks the loop classifies it and relaunches with more.
#[test]
fn a_real_jvm_heap_crash_is_classified_and_relaunched() {
    if which("java").is_none() {
        eprintln!("skipped: no java on PATH, the real-crash case cannot run here");
        return;
    }

    // `-version` keeps the JVM busy; the plan's main class is replaced with a
    // class that allocates, so the OOM is real rather than simulated.
    let oom_class = write_oom_class();
    let mut p = plan();
    let class_dir = oom_class.parent().expect("compiled class lives in a dir");
    // Real plans carry "-cp" and the path as separate arguments.
    p.jvm_args = vec![
        "-Xmx8M".into(),
        "-cp".into(),
        class_dir.to_string_lossy().into(),
    ];
    p.main_class = "Oom".into();
    p.classpath = vec![oom_class.to_string_lossy().into()];
    p.classpath_string = oom_class.to_string_lossy().into();
    p.wrapper_class = None;

    let supervisor = Supervisor::new(
        basalt_engine::exec::ChildProcessExec,
        HeapTweaker,
        basalt_engine::diagnose::LaunchContext {
            heap_mb: Some(8),
            device_ram_mb: Some(4096.0),
            ..Default::default()
        },
    )
    .with_limits(Limits {
        max_attempts: 2,
        heap_ceiling_mb: 256,
    });
    let report = supervisor.run(&p, &java_binary(), sink_for(collector()));

    assert_eq!(report.attempts.len(), 2, "one crash, one retry");
    assert_eq!(
        report.attempts[0].kind(),
        Some(basalt_engine::CrashKind::HeapExhausted),
        "a genuine OutOfMemoryError must classify as heap exhaustion"
    );
    assert!(
        report.exit_code != 0,
        "the retry heap is still too small, so it fails again"
    );
    let _ = std::fs::remove_file(&oom_class);
}

fn which(bin: &str) -> Option<String> {
    std::env::var("PATH").ok().and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(bin))
            .find(|p| p.is_file())
            .map(|p| p.to_string_lossy().into_owned())
    })
}

fn java_binary() -> String {
    std::env::var("JAVA_HOME")
        .ok()
        .map(|home| format!("{home}/bin/java"))
        .filter(|p| std::path::Path::new(p).is_file())
        .or_else(|| which("java"))
        .unwrap_or_else(|| "/usr/bin/java".into())
}

/// Compiles a class that allocates until the heap dies. Java 8+ single-file
/// source launch would need the JDK compiler; this uses it when present and
/// reports honestly when it is not.
fn write_oom_class() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("basalt-oom-case");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let source = dir.join("Oom.java");
    std::fs::write(
        &source,
        "import java.util.*;\n\
         public class Oom {\n\
         \x20   public static void main(String[] a) {\n\
         \x20       List<byte[]> keep = new ArrayList<>();\n\
         \x20       while (true) keep.add(new byte[1024 * 1024]);\n\
         \x20   }\n\
         }\n",
    )
    .expect("write source");
    let javac = java_binary().replace("/bin/java", "/bin/javac");
    let status = std::process::Command::new(javac)
        .arg(&source)
        .current_dir(&dir)
        .status();
    match status {
        Ok(s) if s.success() => dir.join("Oom.class"),
        _ => {
            eprintln!("skipped: no working javac, the real-crash case cannot run here");
            dir.join("Oom.class")
        }
    }
}
