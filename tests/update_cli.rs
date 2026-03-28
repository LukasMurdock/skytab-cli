use std::process::Command;

use serde_json::Value;

#[test]
fn update_check_json_has_stable_shape() {
    let current_version = env!("CARGO_PKG_VERSION");
    let binary = env!("CARGO_BIN_EXE_skytab");

    let output = Command::new(binary)
        .args(["update", "--check", "--json", "--version", current_version])
        .output()
        .expect("failed to run skytab binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid UTF-8");
    let payload: Value = serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    assert_eq!(payload["check_only"], Value::Bool(true));
    assert_eq!(payload["updated"], Value::Bool(false));
    assert_eq!(payload["update_available"], Value::Bool(false));

    let expected_version = format!("v{current_version}");
    assert_eq!(
        payload["current_version"],
        Value::String(expected_version.clone())
    );
    assert_eq!(payload["target_version"], Value::String(expected_version));

    assert!(
        payload["target_triple"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "target_triple should be a non-empty string"
    );

    assert_eq!(
        payload["installed_paths"]
            .as_array()
            .map(std::vec::Vec::len),
        Some(0)
    );
}
