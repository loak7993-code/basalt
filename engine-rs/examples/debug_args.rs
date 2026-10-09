//! Debug dump of the 26.3 game args.

use basalt_engine::manifest::{Platform, VersionManifest};
use basalt_engine::plan::{AuthContext, EngineOptions, InstallLayout, PlanBuilder};
use std::path::PathBuf;

fn main() {
    let raw = include_str!("../tests/fixtures/26.3.json");
    let manifest: VersionManifest = serde_json::from_str(raw).unwrap();
    let layout = InstallLayout {
        client_jar: PathBuf::from("/tmp/26.3.jar"),
        libraries_dir: PathBuf::from("/tmp/libraries"),
        natives_dir: PathBuf::from("/tmp/natives"),
        game_dir: PathBuf::from("/tmp/mc"),
        assets_dir: PathBuf::from("/tmp/assets"),
        asset_index_id: "34".into(),
        runtime_home: PathBuf::from("/tmp/runtime"),
    };
    let options = EngineOptions::default();
    let auth = AuthContext {
        player_name: "basalt_test".into(),
        uuid: "11112222333344445555666677778888".into(),
        ..AuthContext::default()
    };
    let plan = PlanBuilder {
        manifest: &manifest,
        platform: Platform::LinuxX8664,
        layout: &layout,
        features: Default::default(),
        options,
        auth,
        version_name: "26.3".into(),
        version_type: "release".into(),
        quick_play: None,
        log_config_path: None,
    }
    .build();
    for arg in &plan.game_args {
        println!("{}", arg);
    }
}
