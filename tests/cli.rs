//! CLI integration tests: run the ublx binary and assert on exit code and output.

use std::process::Command;

fn ublx_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ublx"))
}

#[test]
fn help_exits_zero() {
    let out = ublx_bin().arg("--help").output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("ublx") || stdout.contains("DIR"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn test_mode_in_empty_dir() {
    let tmp = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("ublx_integration_test_dir");
    let _ = std::fs::create_dir_all(&tmp);
    let out = ublx_bin().arg("--test").arg(&tmp).output().unwrap();
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let db = tmp.join(".ublx");
    assert!(db.exists(), "expected .ublx after --test run");
}
