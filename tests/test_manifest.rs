//! dmaf/manifest.yaml：清单解析与 engine 版本范围判定。

mod common;

use dmaf_engine::manifest::{engine_matches, load};

#[test]
fn engine_range_semantics() {
    // 区间、缺省相等、别名运算子、补丁位缺省
    assert!(engine_matches(">=0.3,<0.4", "0.3.0").unwrap());
    assert!(engine_matches(">=0.3,<0.4", "0.3.7").unwrap());
    assert!(!engine_matches(">=0.3,<0.4", "0.4.0").unwrap());
    assert!(!engine_matches(">=0.3,<0.4", "0.2.9").unwrap());
    assert!(engine_matches("0.3", "0.3.0").unwrap(), "裸版本 = 相等");
    assert!(!engine_matches("0.3", "0.3.1").unwrap());
    assert!(engine_matches(">0.2,<=0.3", "0.3.0").unwrap());
    assert!(!engine_matches(">0.2,<=0.3", "0.3.1").unwrap());
    assert!(engine_matches(">=0.3.1", "0.3.2").unwrap());
    assert!(engine_matches("=0.3.0", "0.3.0").unwrap());
}

#[test]
fn engine_range_errors() {
    assert!(engine_matches("", "0.3.0").is_err(), "空范围报错");
    assert!(engine_matches("abc", "0.3.0").is_err());
    assert!(engine_matches(">=", "0.3.0").is_err());
    assert!(engine_matches(">=0.3", "not-a-version").is_err());
    assert!(engine_matches(">=0..3", "0.3.0").is_err());
}

#[test]
fn load_returns_none_without_manifest() {
    let root = common::unique_tmp("manifest-none");
    assert_eq!(load(&root).unwrap(), None);
}

#[test]
fn load_parses_fields_and_ignores_unknown() {
    let root = common::unique_tmp("manifest-parse");
    std::fs::create_dir_all(root.join("dmaf")).unwrap();
    std::fs::write(
        root.join("dmaf/manifest.yaml"),
        "name: sdlc\nversion: 0.1.0\ndescription: d\nengine: \">=0.3,<0.4\"\nharnesses: [opencode, claude]\ndependencies: []\n",
    )
    .unwrap();
    let m = load(&root).unwrap().unwrap();
    assert_eq!(m.name, "sdlc");
    assert_eq!(m.version, "0.1.0");
    assert_eq!(m.engine, ">=0.3,<0.4");
    assert_eq!(m.harnesses, vec!["opencode".to_string(), "claude".to_string()]);
}

#[test]
fn load_rejects_non_mapping_and_invalid_yaml() {
    let root = common::unique_tmp("manifest-bad");
    std::fs::create_dir_all(root.join("dmaf")).unwrap();
    std::fs::write(root.join("dmaf/manifest.yaml"), "- a\n- b\n").unwrap();
    let err = load(&root).unwrap_err();
    assert!(err.contains("顶层必须是映射"), "{err}");

    std::fs::write(root.join("dmaf/manifest.yaml"), "[1,2,3\n").unwrap();
    assert!(load(&root).is_err());
}
