//! Basalt Engine (Rust)
//! Copyright (C) 2026 Leonxlnx (loak7993-code)
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! Basalt's launch engine: parses a Mojang version manifest, filters libraries
//! per platform, builds the classpath, applies memory and GC policy, and
//! assembles the full JVM and game argument plan. The Android host calls into
//! this crate over JNI; the desktop test suite runs the same code natively.

pub mod exec;
pub mod ffi;
pub mod manifest;
pub mod plan;
pub mod policy;
pub mod rules;

pub use manifest::Platform;
pub use plan::{EngineOptions, InstallLayout, LaunchPlan, PlanBuilder};
pub use policy::{default_ram_allocation, gc_policy_from_args, GcPolicy, MemoryPolicy};
pub use rules::{Features, RuleContext};
