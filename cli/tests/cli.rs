//! Run the built `khmer-kbd` binary on the exported sample.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn sample() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/sample")
}

fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khmer-kbd"))
        .arg("--data")
        .arg(sample())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(args: &[&str], stdin: &str) -> String {
    let output = run(args, stdin);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn convert_arguments_and_standard_input() {
    assert_eq!(
        stdout(&["convert", "sok", "sabay", "te"], ""),
        "សុខសប្បាយទេ\n"
    );
    assert_eq!(
        stdout(&["convert"], "orkun\not mean te\n"),
        "អរគុណ\nអត់មានទេ\n"
    );
}

#[test]
fn suggest_with_context() {
    let first = |args: &[&str]| stdout(args, "").lines().next().unwrap().to_owned();
    assert!(first(&["suggest", "bong"]).starts_with("1. បង\t"));
    assert!(first(&["suggest", "--context", "ការ", "bong"]).starts_with("1. បង់\t"));
    assert_eq!(
        stdout(&["suggest", "-n", "2", "orku"], "").lines().count(),
        2
    );
}

#[test]
fn romanize_both_styles() {
    assert_eq!(stdout(&["romanize", "សុខសប្បាយទេ"], ""), "soksabay te\n");
    assert_eq!(
        stdout(&["romanize", "--style", "ungegn", "សុខសប្បាយទេ"], ""),
        "sŏkhsâbbay té\n"
    );
}

#[test]
fn compile_then_load() {
    let out = std::env::temp_dir().join(format!("khmer-kbd-{}.kbd", std::process::id()));
    let printed = stdout(
        &["compile", sample().to_str().unwrap(), out.to_str().unwrap()],
        "",
    );
    assert!(printed.starts_with("wrote "));
    let converted = Command::new(env!("CARGO_BIN_EXE_khmer-kbd"))
        .args(["--data", out.to_str().unwrap(), "convert", "orkun"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(converted.stdout).unwrap(), "អរគុណ\n");
    std::fs::remove_file(out).unwrap();
}

#[test]
fn repl_learns_a_pick() {
    let user = std::env::temp_dir().join(format!("khmer-kbd-picks-{}.tsv", std::process::id()));
    let out = stdout(
        &["--user", user.to_str().unwrap(), "repl"],
        "bong\n:2\nbong\n:q\n",
    );
    assert!(out.contains("bong: 1.បង  2.បង់"));
    assert!(out.contains("learned បង់ for bong"));
    assert!(out.contains("bong: 1.បង់"));
    assert!(std::fs::read_to_string(&user).unwrap().contains("បង់"));
    std::fs::remove_file(user).unwrap();
}

#[test]
fn bad_usage() {
    assert_eq!(run(&["nonsense"], "").status.code(), Some(1));
    assert_eq!(
        run(&["romanize", "--style", "fancy", "ទេ"], "")
            .status
            .code(),
        Some(1)
    );
    let output = Command::new(env!("CARGO_BIN_EXE_khmer-kbd"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr).unwrap().contains("usage:"));
}
