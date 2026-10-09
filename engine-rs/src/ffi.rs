//! JNI surface for the Android host.
//!
//! Strings in, strings out: the host passes the manifest, layout, options, and
//! auth as JSON; the engine returns the plan as JSON. All engine logic lives
//! in Rust; the host only executes the plan.

use jni::objects::{JClass, JString};
use jni::sys::jint;
use jni::JNIEnv;

use crate::manifest::{Platform, VersionManifest};
use crate::plan::{AuthContext, EngineOptions, InstallLayout, PlanBuilder};

fn to_string(env: &mut JNIEnv, input: &JString) -> Result<String, String> {
    Ok(env.get_string(input).map_err(|e| e.to_string())?.into())
}

fn out<'local>(env: &JNIEnv<'local>, value: Result<String, String>) -> JString<'local> {
    let json = match value {
        Ok(json) => json,
        Err(message) => serde_json::json!({ "error": message }).to_string(),
    };
    env.new_string(json).expect("jstring allocation failed")
}

/// Parses a manifest and returns basic diagnostics as JSON, so the host can
/// verify the Rust engine is live without launching anything.
#[no_mangle]
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeProbe<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    manifest_json: JString<'local>,
) -> JString<'local> {
    let value = (|| -> Result<serde_json::Value, String> {
        let input = to_string(&mut env, &manifest_json)?;
        let manifest: VersionManifest =
            serde_json::from_str(&input).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "id": manifest.id,
            "mainClass": manifest.main_class,
            "libraries": manifest.libraries.len(),
            "requiredJavaMajor": manifest
                .java_version
                .as_ref()
                .map(|j| j.major_version),
        }))
    })();
    out(&env, value.map(|v| v.to_string()))
}

/// Builds the full launch plan.
///
/// Inputs (all JSON strings): manifest, layout (InstallLayout paths), options
/// (EngineOptions), auth (AuthContext), quickPlay (nullable), the target arch,
/// and the version name. Returns the LaunchPlan as JSON.
#[no_mangle]
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeBuildPlan<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    manifest_json: JString<'local>,
    layout_json: JString<'local>,
    options_json: JString<'local>,
    auth_json: JString<'local>,
    quick_play_json: JString<'local>,
    platform_arch: JString<'local>,
    version_name: JString<'local>,
) -> JString<'local> {
    let value = (|| -> Result<String, String> {
        let manifest_str = to_string(&mut env, &manifest_json)?;
        let layout_str = to_string(&mut env, &layout_json)?;
        let options_str = to_string(&mut env, &options_json)?;
        let auth_str = to_string(&mut env, &auth_json)?;
        let qp_str = to_string(&mut env, &quick_play_json)?;
        let arch = to_string(&mut env, &platform_arch)?;
        let version = to_string(&mut env, &version_name)?;

        let manifest: VersionManifest =
            serde_json::from_str(&manifest_str).map_err(|e| format!("manifest: {}", e))?;
        let layout: InstallLayout =
            serde_json::from_str(&layout_str).map_err(|e| format!("layout: {}", e))?;
        let options: EngineOptions =
            serde_json::from_str(&options_str).map_err(|e| format!("options: {}", e))?;
        let auth: AuthContext =
            serde_json::from_str(&auth_str).map_err(|e| format!("auth: {}", e))?;
        let quick_play: Option<crate::plan::QuickPlay> =
            serde_json::from_str(&qp_str).map_err(|e| format!("quickPlay: {}", e))?;
        let platform = Platform::from("linux", &arch)
            .ok_or_else(|| format!("unsupported platform: linux/{}", arch))?;

        let plan = PlanBuilder {
            manifest: &manifest,
            platform,
            layout: &layout,
            features: crate::rules::Features::default(),
            options,
            auth,
            version_name: version,
            version_type: "release".into(),
            quick_play,
            log_config_path: None,
        }
        .build();
        serde_json::to_string(&plan).map_err(|e| e.to_string())
    })();
    out(&env, value)
}

/// Applies the engine GC policy to an existing arg list (JSON array in, array
/// out). Mirrors gcPolicyFromArgs + applyTo.
#[no_mangle]
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeApplyGcPolicy<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    args_json: JString<'local>,
) -> JString<'local> {
    let value = (|| -> Result<String, String> {
        let input = to_string(&mut env, &args_json)?;
        let args: Vec<String> = serde_json::from_str(&input).map_err(|e| e.to_string())?;
        let policy = crate::policy::gc_policy_from_args(&args);
        let mut out = args;
        policy.apply_to(&mut out);
        serde_json::to_string(&out).map_err(|e| e.to_string())
    })();
    out(&env, value)
}

/// The device-proportional memory default in Rust: 55% of RAM, floor 1536 MB
/// within device limits. Returns the value in MB.
#[no_mangle]
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeDefaultRamAllocation<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    device_ram_mb: jint,
    is_32_bit: jint,
) -> jint {
    crate::policy::default_ram_allocation(device_ram_mb as f64, is_32_bit != 0) as jint
}
