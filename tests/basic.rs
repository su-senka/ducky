use assert_cmd::Command;

#[test]
fn shows_help() {
    let mut cmd = Command::cargo_bin("ducky").unwrap();
    cmd.arg("--help").assert().success();
}

#[test]
fn json_mode_emits_pure_array() {
    let mut cmd = Command::cargo_bin("ducky").unwrap();
    cmd.arg(".").arg("--json");
    let assert = cmd.assert().success();
    let output = assert.get_output();
    assert!(output.status.success());
    // stdout should start with [ in JSON array mode
    assert!(output.stdout.starts_with(b"["), "stdout should begin with '[' in --json mode");
}
