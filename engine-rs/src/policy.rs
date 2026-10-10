//! Launch policies: memory and garbage collection.

/// Memory policy for the game JVM. The caller computes the device default;
/// the engine clamps it into a sane window so a bad reading can never produce
/// a 256 MB heap on a device that supports more.
#[derive(Debug, Clone)]
pub struct MemoryPolicy {
    pub heap_mb: i64,
    pub min_mb: i64,
    pub max_mb: i64,
}

impl MemoryPolicy {
    pub fn effective_heap_mb(&self) -> i64 {
        self.heap_mb.clamp(self.min_mb, self.max_mb)
    }

    /// Replaces any manifest-provided heap flags with the policy's own.
    pub fn apply_heap_to(&self, args: &mut Vec<String>) {
        args.retain(|arg| !arg.starts_with("-Xms") && !arg.starts_with("-Xmx"));
        let heap = self.effective_heap_mb();
        args.push(format!("-Xms{}M", heap));
        args.push(format!("-Xmx{}M", heap));
    }
}

/// Garbage collection policy.
#[derive(Debug, Clone, PartialEq)]
pub enum GcPolicy {
    /// The phone-class tuned baseline.
    TunedG1,
    /// Leave the JVM default.
    None,
    /// The user already picked flags; pass through untouched.
    Custom(Vec<String>),
}

/// The tuned G1 baseline for phone-class heaps.
pub const TUNED_G1_FLAGS: [&str; 7] = [
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UseG1GC",
    "-XX:G1NewSizePercent=20",
    "-XX:G1ReservePercent=20",
    "-XX:MaxGCPauseMillis=40",
    "-XX:G1HeapRegionSize=16M",
    "-XX:+DisableExplicitGC",
];

/// Flags a GC policy owns when it overrides manifest defaults: collector
/// selection and its tuning knobs. Object-layout flags such as
/// -XX:+UseCompactObjectHeaders are NOT owned by the GC policy and survive.
pub const GC_OWNED_PREFIXES: [&str; 17] = [
    "-XX:+UseG1GC",
    "-XX:+UseZGC",
    "-XX:+UseSerialGC",
    "-XX:+UseParallelGC",
    "-XX:+UseParallelOldGC",
    "-XX:+UseConcMarkSweepGC",
    "-XX:+UseShenandoahGC",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+DisableExplicitGC",
    "-XX:G1NewSizePercent=",
    "-XX:G1MaxNewSizePercent=",
    "-XX:G1ReservePercent=",
    "-XX:MaxGCPauseMillis=",
    "-XX:G1HeapRegionSize=",
    "-XX:G1HeapWastePercent=",
    "-XX:G1MixedGCCountTarget=",
    "-XX:InitiatingHeapOccupancyPercent=",
];

/// Flags the heap policy owns: the launcher decides memory, not the manifest.
pub const HEAP_OWNED_PREFIXES: [&str; 2] = ["-Xms", "-Xmx"];

impl GcPolicy {
    /// Applies the collector policy, replacing manifest-provided ones.
    pub fn apply_to(&self, args: &mut Vec<String>) {
        match self {
            GcPolicy::TunedG1 => {
                args.retain(|arg| !GC_OWNED_PREFIXES.iter().any(|p| arg.starts_with(p)));
                args.extend(TUNED_G1_FLAGS.iter().map(|s| s.to_string()));
            }
            GcPolicy::None => {}
            GcPolicy::Custom(_) => {}
        }
    }
}

/// Picks the policy from an existing arg list: any user-selected GC wins,
/// otherwise the tuned baseline applies.
pub fn gc_policy_from_args(args: &[String]) -> GcPolicy {
    let user_gc = args
        .iter()
        .any(|arg| arg.starts_with("-XX:+Use") && arg.ends_with("GC"));
    if user_gc {
        GcPolicy::Custom(
            args.iter()
                .filter(|arg| arg.starts_with("-XX:"))
                .cloned()
                .collect(),
        )
    } else {
        GcPolicy::TunedG1
    }
}

/// Device-proportional memory default: about 55% of total RAM, floor 1536 MB
/// where the device allows, capped by the device max (mirrors the Kotlin
/// engine's getDefaultRamAllocation).
pub fn default_ram_allocation(device_ram_mb: f64, is_32_bit: bool) -> i64 {
    let max: f64 = if is_32_bit || device_ram_mb < 2048.0 {
        1024.0_f64.min(device_ram_mb)
    } else {
        device_ram_mb
            - if device_ram_mb < 3064.0 {
                800.0
            } else {
                1024.0
            }
    };
    let target = (device_ram_mb * 0.55) as i64;
    let floor = 1536_i64.min(max as i64);
    target.max(floor).min(max as i64)
}
