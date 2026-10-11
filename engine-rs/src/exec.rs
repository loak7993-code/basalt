//! Execution strategies. The reference implementation spawns a real java
//! process; Android hosts bind an in-process exec through their native bridge.

use crate::plan::LaunchPlan;

pub trait ExecStrategy {
    /// Runs the plan and streams each output line to the sink.
    /// Returns the process exit code.
    fn exec(&self, plan: &LaunchPlan, java_binary: &str, sink: &mut dyn FnMut(&str)) -> i32;
}

/// Reference exec: a child java process with merged output. The Android host
/// uses its own in-process strategy; this one exists for desktop hosts and
/// tests.
pub struct ChildProcessExec;

impl ExecStrategy for ChildProcessExec {
    fn exec(&self, plan: &LaunchPlan, java_binary: &str, sink: &mut dyn FnMut(&str)) -> i32 {
        let command = plan.to_command_list(java_binary);
        let mut process = std::process::Command::new(&command[0])
            .args(&command[1..])
            .stdout(std::process::Stdio::piped())
            // The JVM reports OutOfMemoryError, UnsupportedClassVersionError and
            // bad-flag failures on stderr. Dropping it would hide exactly the
            // lines crash diagnosis runs on, so both streams are drained.
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn the java process");
        let stdout = process.stdout.take().expect("stdout was piped");
        let stderr = process.stderr.take().expect("stderr was piped");

        // One reader per pipe feeding one channel, so neither pipe can fill up
        // and deadlock the other. Ordering between the two streams is best
        // effort; every line still reaches the sink.
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        std::thread::scope(|scope| {
            let pump = |pipe: Box<dyn std::io::Read + Send>,
                        tx: std::sync::mpsc::Sender<String>| {
                scope.spawn(move || {
                    use std::io::BufRead;
                    for line in std::io::BufReader::new(pipe).lines().map_while(Result::ok) {
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                });
            };
            pump(Box::new(stdout), tx.clone());
            pump(Box::new(stderr), tx.clone());
            drop(tx);
            for line in rx {
                sink(&line);
            }
        });

        match process.wait() {
            Ok(status) => status.code().unwrap_or(-1),
            Err(_) => -1,
        }
    }
}
