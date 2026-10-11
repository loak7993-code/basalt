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
        let manifest: VersionManifest = serde_json::from_str(&input).map_err(|e| e.to_string())?;
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
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeApplyGcPolicy<
    'local,
>(
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
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeDefaultRamAllocation<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    device_ram_mb: jint,
    is_32_bit: jint,
) -> jint {
    crate::policy::default_ram_allocation(device_ram_mb as f64, is_32_bit != 0) as jint
}

/// Starts a download job. tasks_json: array of
/// {urls:[...], target:"...", sha1:..., size:n}. Returns the job id.
#[no_mangle]
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeDownloadStart<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    tasks_json: JString<'local>,
    workers: jint,
) -> JString<'local> {
    let value = (|| -> Result<String, String> {
        let input = to_string(&mut env, &tasks_json)?;
        let specs: Vec<crate::download::DownloadSpec> =
            serde_json::from_str(&input).map_err(|e| e.to_string())?;
        let id = crate::download::download_start(specs, workers.max(1) as usize);
        Ok(id.to_string())
    })();
    out(&env, value)
}

/// Progress snapshot for a job: {done,total,bytesDone,bytesTotal,finished}.
#[no_mangle]
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeDownloadProgress<
    'local,
>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    job_id: jint,
) -> JString<'local> {
    let snap = crate::download::download_progress(job_id as u32).unwrap_or_default();
    out(&env, Ok(serde_json::to_string(&snap).unwrap_or_default()))
}

/// Blocks until the job finishes; returns {failures:[{path,error}]}.
#[no_mangle]
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeDownloadWait<
    'local,
>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    job_id: jint,
) -> JString<'local> {
    let value = (|| -> Result<String, String> {
        let failures = crate::download::download_wait(job_id as u32)
            .ok_or_else(|| "unknown job".to_string())?;
        serde_json::to_string(&failures).map_err(|e| e.to_string())
    })();
    out(&env, value)
}

/// Classifies a failed launch from its log lines. `facts_json` carries the
/// host's knowledge of the launch:
/// `{"heapMb":1637,"deviceRamMb":4096,"renderer":"gl4es","javaMajor":25,"modsDir":"/..."}`
/// Lines may be a JSON array of strings or one newline-separated string.
/// Returns JSON: {kind, headline, fix, evidence:[...]}.
#[no_mangle]
pub extern "system" fn Java_com_movtery_zalithlauncher_engine_RustEngine_nativeDiagnoseCrash<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    lines_json: JString<'local>,
    exit_code: jint,
    facts_json: JString<'local>,
) -> JString<'local> {
    let value = (|| -> Result<String, String> {
        let raw = to_string(&mut env, &lines_json)?;
        let facts_raw = to_string(&mut env, &facts_json)?;
        let lines: Vec<String> = match serde_json::from_str::<Vec<String>>(&raw) {
            Ok(list) => list,
            Err(_) => raw.lines().map(str::to_string).collect(),
        };
        let facts: DiagnoseFacts = serde_json::from_str(&facts_raw).unwrap_or_default();
        let diagnosis = crate::diagnose::diagnose(&lines, exit_code, &facts.context());
        let fix = match &diagnosis.fix {
            crate::diagnose::Fix::SetHeap { heap_mb, reason } => serde_json::json!({
                "type": "setHeap", "heapMb": heap_mb, "reason": reason
            }),
            crate::diagnose::Fix::SetRenderer { renderer } => serde_json::json!({
                "type": "setRenderer", "renderer": renderer.as_str()
            }),
            crate::diagnose::Fix::UseJava { major } => serde_json::json!({
                "type": "useJava", "major": major
            }),
            crate::diagnose::Fix::RefetchFiles { paths } => serde_json::json!({
                "type": "refetch", "paths": paths
            }),
            crate::diagnose::Fix::DisableMods { files } => serde_json::json!({
                "type": "disableMods", "files": files.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<String>>()
            }),
            crate::diagnose::Fix::None { reason } => serde_json::json!({
                "type": "none", "reason": reason
            }),
        };
        Ok(serde_json::json!({
            "kind": diagnosis.kind.as_str(),
            "headline": diagnosis.kind.headline(),
            "exitCode": diagnosis.exit_code,
            "fix": fix,
            "fixDescription": diagnosis.fix.describe(),
            "evidence": diagnosis.evidence,
            "tail": diagnosis.tail,
        })
        .to_string())
    })();
    out(&env, value)
}

/// Host-supplied facts about the launch that failed.
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiagnoseFacts {
    heap_mb: Option<i64>,
    device_ram_mb: Option<f64>,
    renderer: Option<String>,
    java_major: Option<i64>,
    mods_dir: Option<String>,
}

impl DiagnoseFacts {
    fn context(&self) -> crate::diagnose::LaunchContext {
        crate::diagnose::LaunchContext {
            heap_mb: self.heap_mb,
            device_ram_mb: self.device_ram_mb,
            renderer: self
                .renderer
                .as_deref()
                .and_then(crate::diagnose::Renderer::parse),
            java_major: self.java_major,
            mods_dir: self.mods_dir.as_ref().map(std::path::PathBuf::from),
        }
    }
}
