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
            // Merged stream: no interleaved-pipe deadlock, single drain order.
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("failed to spawn the java process");
        let stdout = process.stdout.take().expect("stdout was piped");
        use std::io::BufRead;
        for line in std::io::BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
        {
            sink(&line);
        }
        match process.wait() {
            Ok(status) => status.code().unwrap_or(-1),
            Err(_) => -1,
        }
    }
}
