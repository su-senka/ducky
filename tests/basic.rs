use assert_cmd::Command;

#[test]
fn shows_help() {
    let mut cmd = Command::cargo_bin("ducky").unwrap();
    cmd.arg("--help").assert().success();
}
