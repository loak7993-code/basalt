//! The supervised launch loop.
//!
//! Copyright (C) 2026 Leonxlnx (loak7993-code)
//!
//! This program is free software: you can redistribute it and/or modify it
//! under the terms of the GNU General Public License as published by the Free
//! Software Foundation, either version 3 of the License, or (at your option)
//! any later version.
//!
//! This program is distributed in the hope that it will be useful, but WITHOUT
//! ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
//! FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
//! more details.
//!
//! You should have received a copy of the GNU General Public License along
//! with this program. If not, see <https://www.gnu.org/licenses/gpl-3.0.html>.

//! Runs a launch, watches its log stream, and on failure classifies the
//! crash and relaunches once with a fix applied. The loop is deliberately
//! bounded: it never applies the same fix twice and never relaunches more
//! than `max_attempts` times, so a broken configuration surfaces as a report
//! instead of a fork bomb.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use crate::diagnose::{self, CrashKind, Diagnosis, Fix, LaunchContext, Renderer};
use crate::exec::ExecStrategy;
use crate::plan::LaunchPlan;

/// One attempt's worth of outcome, in launch order.
#[derive(Debug, Clone)]
pub struct Attempt {
    /// What came back from the JVM.
    pub exit_code: i32,
    /// The classification, present only for failed attempts.
    pub diagnosis: Option<Diagnosis>,
    /// Files this attempt moved aside, so the caller can undo them.
    pub disabled_files: Vec<PathBuf>,
}

impl Attempt {
    pub fn diagnosis(&self) -> Option<&Diagnosis> {
        self.diagnosis.as_ref()
    }

    pub fn kind(&self) -> Option<CrashKind> {
        self.diagnosis.as_ref().map(|d| d.kind)
    }
}

/// Everything a supervised launch produced.
#[derive(Debug, Clone)]
pub struct LaunchReport {
    pub attempts: Vec<Attempt>,
    /// Exit code of the last attempt, which is the one that ended the run.
    pub exit_code: i32,
}

impl LaunchReport {
    /// A launch that worked on the first try.
    pub fn succeeded(&self) -> bool {
        self.attempts
            .first()
            .map(|a| a.exit_code == 0)
            .unwrap_or(false)
    }

    /// A launch that needed at least one remediation to get going.
    pub fn recovered(&self) -> bool {
        self.attempts.len() > 1 && self.exit_code == 0
    }

    pub fn diagnosis(&self) -> Option<&Diagnosis> {
        self.attempts
            .iter()
            .rev()
            .find_map(|a| a.diagnosis.as_ref())
    }

    /// Every file the loop moved aside, newest attempt last.
    pub fn disabled_files(&self) -> Vec<PathBuf> {
        self.attempts
            .iter()
            .flat_map(|a| a.disabled_files.iter().cloned())
            .collect()
    }
}

/// How the loop reports progress. Every line the game prints is forwarded, so
/// a supervised launch looks exactly like an unsupervised one on the console.
pub type LineSink = Arc<dyn Fn(&str) + Send + Sync>;

/// Host hook that re-downloads the files a corrupt-asset diagnosis named.
pub type Refetch = Arc<dyn Fn(&[String]) -> bool + Send + Sync>;

/// Bounds on what the loop is allowed to do without a human.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Total launches, including the first one.
    pub max_attempts: u32,
    /// Heap the loop may never plan above, regardless of the device.
    pub heap_ceiling_mb: i64,
}

impl Default for Limits {
    fn default() -> Self {
        // Two relaunches: enough for "heap was too small, then the fix exposed
        // a mod problem", not enough to spin.
        Self {
            max_attempts: 3,
            heap_ceiling_mb: 8192,
        }
    }
}

/// How the launch should be adjusted between attempts.
pub trait PlanTweaker: Send + Sync {
    /// Returns true when the tweak was applied.
    fn set_heap(&self, plan: &mut LaunchPlan, heap_mb: i64) -> bool;
    /// Renderer choice is a host setting on Android; desktop has no knob.
    fn set_renderer(&self, plan: &mut LaunchPlan, renderer: Renderer) -> bool {
        let _ = (plan, renderer);
        false
    }
    /// Refetching files belongs to the download engine, which the caller owns.
    fn refetch(&self, files: &[String]) -> bool {
        let _ = files;
        false
    }
}

/// The default tweaker: heap only, which is the one fix a launch plan owns.
pub struct HeapTweaker;

impl PlanTweaker for HeapTweaker {
    fn set_heap(&self, plan: &mut LaunchPlan, heap_mb: i64) -> bool {
        // The plan carries the value attached ("-Xmx1637M"), so match the
        // prefix. Appending a second -Xmx would leave the first one winning.
        let idx = plan
            .jvm_args
            .iter()
            .position(|a| a.starts_with("-Xmx") || a.starts_with("-XX:MaxHeapSize="));
        match idx {
            Some(i) if plan.jvm_args[i].starts_with("-XX:MaxHeapSize=") => {
                plan.jvm_args[i] = format!("-XX:MaxHeapSize={heap_mb}M");
                true
            }
            Some(i) => {
                plan.jvm_args[i] = format!("-Xmx{heap_mb}M");
                true
            }
            None => {
                plan.jvm_args.push(format!("-Xmx{heap_mb}M"));
                true
            }
        }
    }
}

/// The mutable parts of the launch the loop adjusts between attempts.
pub struct Supervisor<E: ExecStrategy, T: PlanTweaker> {
    exec: E,
    tweaker: T,
    limits: Limits,
    /// Facts about the launch, refreshed by the caller between attempts.
    ctx: LaunchContext,
    /// Disabled mod files across all attempts.
    disabled: Mutex<Vec<PathBuf>>,
    /// Re-fetch hook supplied by the host (the download engine).
    refetch: Option<Refetch>,
}

impl<E: ExecStrategy, T: PlanTweaker> Supervisor<E, T> {
    pub fn new(exec: E, tweaker: T, ctx: LaunchContext) -> Self {
        Self {
            exec,
            tweaker,
            limits: Limits::default(),
            ctx,
            disabled: Mutex::new(Vec::new()),
            refetch: None,
        }
    }

    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_refetch(mut self, hook: Refetch) -> Self {
        self.refetch = Some(hook);
        self
    }

    /// Updates the facts the next diagnosis will use.
    pub fn set_context(&mut self, ctx: LaunchContext) {
        self.ctx = ctx;
    }

    /// Runs the plan, fixing and relaunching on failure.
    ///
    /// `sink` receives every line the game prints, from every attempt.
    pub fn run(&self, plan: &LaunchPlan, java_binary: &str, sink: LineSink) -> LaunchReport {
        let mut plan = plan.clone();
        let mut attempts: Vec<Attempt> = Vec::new();
        // `applied` is only ever written by this one call, so an interior cell
        // keeps the rest of the supervisor immutable for its callers.
        let applied = Mutex::new(BTreeSet::new());
        // Facts used by the next diagnosis. Updated when a fix changes one, so
        // a second OOM is judged against the heap the retry actually used.
        let mut ctx = self.ctx.clone();

        for attempt_no in 1..=self.limits.max_attempts {
            let captured: Mutex<Vec<String>> = Mutex::new(Vec::new());
            let collector = {
                let captured = &captured;
                let sink = Arc::clone(&sink);
                move |line: &str| {
                    if let Ok(mut buffer) = captured.lock() {
                        buffer.push(line.to_string());
                    }
                    sink(line);
                }
            };
            let mut collector = collector;
            let exit_code = self.exec.exec(&plan, java_binary, &mut collector);
            let captured = captured.into_inner().unwrap_or_default();

            if exit_code == 0 && !has_fatal_marker(&captured) {
                attempts.push(Attempt {
                    exit_code,
                    diagnosis: None,
                    disabled_files: self.disabled.lock().map(|d| d.clone()).unwrap_or_default(),
                });
                return LaunchReport {
                    attempts,
                    exit_code,
                };
            }

            let diagnosis = diagnose::diagnose(&captured, exit_code, &ctx);
            let key = fix_key(&diagnosis.fix);

            // Never the same remediation twice: a loop that keeps raising the
            // heap is worse than a launch that fails once with a report.
            let already_applied = applied.lock().map(|set| set.contains(&key)).unwrap_or(true);
            let can_retry = attempt_no < self.limits.max_attempts
                && !key.is_empty()
                && !already_applied
                && self.apply(&diagnosis.fix, &mut plan, &mut ctx);

            attempts.push(Attempt {
                exit_code,
                diagnosis: Some(diagnosis.clone()),
                disabled_files: self.disabled.lock().map(|d| d.clone()).unwrap_or_default(),
            });

            if !can_retry {
                return LaunchReport {
                    attempts,
                    exit_code,
                };
            }
            if let Ok(mut set) = applied.lock() {
                set.insert(key);
            }
            sink(&diagnosis.fix.describe());
        }

        let exit_code = attempts.last().map(|a| a.exit_code).unwrap_or(-1);
        LaunchReport {
            attempts,
            exit_code,
        }
    }

    /// Applies one remediation to the plan. Returns false when the host cannot
    /// carry out the fix, which ends the loop rather than repeating it.
    fn apply(&self, fix: &Fix, plan: &mut LaunchPlan, ctx: &mut LaunchContext) -> bool {
        match fix {
            Fix::SetHeap { heap_mb, .. } => {
                let target = match heap_mb {
                    Some(mb) => *mb,
                    // No heap was set at all: give the engine's own default.
                    None => match plan.jvm_args.iter().position(|a| a == "-Xmx") {
                        Some(_) => return false,
                        None => crate::policy::default_ram_allocation(4096.0, false),
                    },
                };
                let target = target.min(self.limits.heap_ceiling_mb);
                let applied = self.tweaker.set_heap(plan, target);
                if applied {
                    ctx.heap_mb = Some(target);
                }
                applied
            }
            Fix::SetRenderer { renderer } => self.tweaker.set_renderer(plan, *renderer),
            Fix::RefetchFiles { paths } => match self.refetch.as_ref() {
                Some(hook) => hook(paths),
                None => false,
            },
            Fix::DisableMods { files } => match diagnose::disable_mods(files) {
                Ok(renamed) => {
                    if let Ok(mut guard) = self.disabled.lock() {
                        guard.extend(renamed);
                    }
                    true
                }
                Err(_) => false,
            },
            // Nothing to apply: either no safe fix, or the host refused one.
            Fix::UseJava { .. } | Fix::None { .. } => false,
        }
    }
}

/// Identity of a fix, used to make sure the loop never repeats one.
fn fix_key(fix: &Fix) -> String {
    match fix {
        Fix::SetHeap { .. } => "heap".to_string(),
        Fix::SetRenderer { renderer } => format!("renderer:{}", renderer.as_str()),
        Fix::UseJava { major } => format!("java:{major}"),
        Fix::RefetchFiles { .. } => "refetch".to_string(),
        Fix::DisableMods { files } => {
            let names: Vec<String> = files
                .iter()
                .map(|f| f.file_name().unwrap_or_default().to_string_lossy().into())
                .collect();
            format!("disable:{}", names.join(","))
        }
        Fix::None { .. } => String::new(),
    }
}

/// The game sometimes logs a fatal error and still exits 0. Those launches are
/// failures, and the loop has to see them.
pub fn has_fatal_marker(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("fatal") || lower.contains("exception in thread \"main\"")
    })
}

/// Restores every file the report says was moved aside.
pub fn undo(report: &LaunchReport) -> std::io::Result<Vec<PathBuf>> {
    diagnose::enable_mods(&report.disabled_files())
}
