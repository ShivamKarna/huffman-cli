//! End to end tests that run the compiled `huff` binary.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_huff"))
}

/// Creates a fresh scratch directory unique to this test.
fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("huff_cli_{}_{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn no_args_prints_usage_and_fails() {
    let out = bin().output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("missing arguments"));
}

#[test]
fn missing_filename_fails() {
    let out = bin().arg("-en").output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("missing filename"));
}

#[test]
fn unknown_flag_fails() {
    let out = bin().args(["-xx", "a.txt"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown flag"));
}

#[test]
fn wrong_extension_fails() {
    let out = bin().args(["-en", "notes.md"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid file extension"));
}

#[test]
fn missing_input_file_shows_cause() {
    let out = bin().args(["-en", "no_such_file.txt"]).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("I/O error"));
    assert!(stderr.contains("caused by"));
}

#[test]
fn encode_then_decode_through_the_cli() {
    let dir = scratch_dir("roundtrip");

    let txt = dir.join("note.txt");
    std::fs::write(&txt, "abracadabra").unwrap();

    let enc = bin().arg("-en").arg(&txt).output().unwrap();
    assert!(enc.status.success());

    let huff = dir.join("note.en.huff");
    let dec = bin().arg("-den").arg(&huff).output().unwrap();
    assert!(dec.status.success());

    let decoded = std::fs::read_to_string(dir.join("note.dec.txt")).unwrap();
    assert_eq!(decoded, "abracadabra");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn second_encode_refuses_to_overwrite() {
    let dir = scratch_dir("overwrite");

    let txt = dir.join("note.txt");
    std::fs::write(&txt, "hello").unwrap();

    assert!(bin()
        .arg("-en")
        .arg(&txt)
        .output()
        .unwrap()
        .status
        .success());

    let again = bin().arg("-en").arg(&txt).output().unwrap();
    assert!(!again.status.success());
    assert!(String::from_utf8_lossy(&again.stderr).contains("already exists"));

    std::fs::remove_dir_all(&dir).unwrap();
}
