use nefaxer::NefaxOpts;
use zahirscan::RuntimeConfig;

use ublx::config::LayoutOverlay;
use ublx::config::{EnhancePolicy, EnhancePolicyEntry, UblxOpts, UblxOverlay};

fn opts_with(enable_enhance_all: bool, entries: Vec<EnhancePolicyEntry>) -> UblxOpts {
    UblxOpts {
        nefax: NefaxOpts::default(),
        zahir: RuntimeConfig::new(),
        max_workers_available: 1,
        nefax_workers_override: None,
        zahir_workers_override: None,
        ublx_workers_override: None,
        streaming: false,
        config_source: None,
        theme: None,
        transparent: false,
        layout: LayoutOverlay::default(),
        editor_path: None,
        enable_enhance_all,
        enable_enhance_all_cache_before_apply: None,
        enhance_policy: entries,
    }
}

#[test]
fn manual_overrides_global_on() {
    let o = opts_with(
        true,
        vec![EnhancePolicyEntry {
            path: "blocked".into(),
            policy: EnhancePolicy::Manual,
        }],
    );
    assert!(!o.batch_zahir_for_path("blocked/file.txt"));
    assert!(o.batch_zahir_for_path("other/file.txt"));
}

#[test]
fn auto_overrides_global_off() {
    let o = opts_with(
        false,
        vec![EnhancePolicyEntry {
            path: "force".into(),
            policy: EnhancePolicy::Auto,
        }],
    );
    assert!(o.batch_zahir_for_path("force/a.rs"));
    assert!(!o.batch_zahir_for_path("outside/a.rs"));
}

#[test]
fn longest_prefix_wins() {
    let o = opts_with(
        true,
        vec![
            EnhancePolicyEntry {
                path: "a".into(),
                policy: EnhancePolicy::Manual,
            },
            EnhancePolicyEntry {
                path: "a/b".into(),
                policy: EnhancePolicy::Auto,
            },
        ],
    );
    assert!(!o.batch_zahir_for_path("a/x"));
    assert!(o.batch_zahir_for_path("a/b/x"));
}

#[test]
fn deserializes_legacy_always_never_toml() {
    let s = r#"
        [[enhance_policy]]
        path = "legacy"
        policy = "always"
        [[enhance_policy]]
        path = "legacy2"
        policy = "never"
    "#;
    let overlay: UblxOverlay = toml::from_str(s).expect("parse");
    let entries = overlay.enhance_policy.expect("entries");
    assert_eq!(entries[0].policy, EnhancePolicy::Auto);
    assert_eq!(entries[1].policy, EnhancePolicy::Manual);
}
