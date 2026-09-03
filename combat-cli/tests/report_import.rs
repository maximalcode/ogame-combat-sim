use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn report_import_requires_explicit_transfer_consent_and_redacts_bad_ids() {
    let output = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .arg("report")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("third-party"));
    assert!(error.contains("cach"));
    let mut child = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["report", "--allow-proxy-transfer"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"https://private.example/secret\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("private.example")
    );
}

#[test]
fn mistaken_positional_ids_are_not_echoed_by_argument_errors() {
    let output = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["report", "private-token"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("private-token")
    );
}
