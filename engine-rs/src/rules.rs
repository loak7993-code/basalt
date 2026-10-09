//! Rule evaluation for Mojang libraries and argument entries.

use crate::manifest::{LibraryEntry, LibraryRule, Platform};

/// Feature flags a launch can enable; mirrors Mojang's documented features.
#[derive(Debug, Clone, Default)]
pub struct Features {
    pub has_custom_resolution: bool,
    pub resolution_width: Option<u32>,
    pub resolution_height: Option<u32>,
    pub is_demo_user: bool,
    pub has_quick_plays_support: bool,
    pub is_quick_play_singleplayer: bool,
    pub is_quick_play_multiplayer: bool,
    pub is_quick_play_realms: bool,
}

impl Features {
    fn named(&self, name: &str) -> Option<bool> {
        match name {
            "has_custom_resolution" => Some(self.has_custom_resolution),
            "is_demo_user" => Some(self.is_demo_user),
            "has_quick_plays_support" => Some(self.has_quick_plays_support),
            "is_quick_play_singleplayer" => Some(self.is_quick_play_singleplayer),
            "is_quick_play_multiplayer" => Some(self.is_quick_play_multiplayer),
            "is_quick_play_realms" => Some(self.is_quick_play_realms),
            _ => None,
        }
    }
}

/// Evaluation context: the target platform plus enabled features.
pub struct RuleContext {
    pub platform: Platform,
    pub features: Features,
}

/// Free-function form: allowed on a specific platform without building a
/// context. Used by the installer's cross-platform download filter.
pub fn is_allowed_on(
    library: &LibraryEntry,
    platform: Platform,
    features: &Features,
) -> bool {
    RuleContext {
        platform,
        features: features.clone(),
    }
    .is_allowed(library)
}

impl RuleContext {
    /// Rules are a conjunction: a rule matches when every condition it
    /// carries is satisfied.
    pub fn rule_matches(&self, rule: &LibraryRule) -> bool {
        if let Some(os) = &rule.os {
            if let Some(name) = &os.name {
                if !name.eq_ignore_ascii_case(self.platform.os_name()) {
                    return false;
                }
            }
            if let Some(arch) = &os.arch {
                if !arch.eq_ignore_ascii_case(self.platform.arch_name()) {
                    return false;
                }
            }
        }
        if let Some(feature_map) = &rule.features {
            for (name, expected) in feature_map {
                // Unknown features are treated as false, matching launcher
                // convention.
                let actual = self.features.named(name).unwrap_or(false);
                if actual != *expected {
                    return false;
                }
            }
        }
        true
    }

    /// A library is allowed when no "allow" rule exists (default allow), or
    /// when at least one allow rule matches and no matching deny rule exists.
    pub fn is_allowed(&self, library: &LibraryEntry) -> bool {
        if library.rules.is_empty() {
            return true;
        }
        let mut any_allow = false;
        for rule in &library.rules {
            let matches = self.rule_matches(rule);
            if matches && rule.action.eq_ignore_ascii_case("allow") {
                any_allow = true;
            }
            if matches && rule.action.eq_ignore_ascii_case("disallow") {
                return false;
            }
        }
        any_allow
    }
}
