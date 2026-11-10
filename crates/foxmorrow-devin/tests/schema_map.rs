use std::path::PathBuf;

use foxmorrow_core::Error;
use foxmorrow_devin::schema_map::{SCHEMA_MAP_FILE, SchemaMap};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn mutate(change: impl FnOnce(&mut serde_json::Value)) -> foxmorrow_core::Result<SchemaMap> {
    let text = std::fs::read_to_string(root().join(SCHEMA_MAP_FILE)).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    change(&mut value);
    SchemaMap::parse(&value.to_string())
}

#[test]
fn shipped_map_loads() {
    let map = SchemaMap::load(&root()).unwrap();
    assert_eq!(map.map_version, 1);
    assert!(map.tested_devin.contains(&"3000.12.43".to_string()));
    assert_eq!(map.message.model, "/metadata/generation_model");
}

#[test]
fn rejects_broken_maps() {
    assert!(matches!(
        mutate(|v| v["map_version"] = 2.into()),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        mutate(|v| v["message"]["role"] = "role".into()),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        mutate(|v| {
            v["tables"]
                .as_object_mut()
                .unwrap()
                .remove("subagent_heads");
        }),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        mutate(|v| {
            v["tables"]["x; DROP TABLE y"] = serde_json::json!(["a"]);
        }),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        mutate(|v| v["subagent_result_prefix"] = " ".into()),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        mutate(|v| v["extra"] = 1.into()),
        Err(Error::Json(_))
    ));
    assert!(matches!(
        SchemaMap::load(&root().join("nowhere")),
        Err(Error::NotFound(_))
    ));
}
