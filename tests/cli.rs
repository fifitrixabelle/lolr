use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn lolr() -> Command {
    Command::cargo_bin("lolr").unwrap()
}

#[test]
fn help_works() {
    lolr()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rainbow colorizer"));
}

#[test]
fn version_works() {
    lolr()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("lolr"));
}

#[test]
fn ruby_compatible_short_version_works() {
    lolr()
        .arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::contains("lolr"));
}

#[test]
fn stdin_produces_ansi() {
    lolr()
        .arg("--force")
        .write_stdin("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b["));
}

#[test]
fn gradient_option_works() {
    lolr()
        .args(["--force", "--gradient", "fire"])
        .write_stdin("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b["));
}

#[test]
fn spread_option_works() {
    lolr()
        .args(["--force", "--spread", "5.0"])
        .write_stdin("test")
        .assert()
        .success();
}

#[test]
fn freq_option_works() {
    lolr()
        .args(["--force", "--freq", "0.2"])
        .write_stdin("test")
        .assert()
        .success();
}

#[test]
fn rejects_invalid_gradient() {
    lolr()
        .args(["--force", "--gradient", "wat"])
        .write_stdin("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown gradient"));
}

#[test]
fn rejects_out_of_range_animation_values() {
    for args in [["--spread", "0"], ["--speed", "0"], ["--duration", "0"]] {
        lolr().args(args).assert().failure();
    }
}

#[test]
fn force_preserves_missing_final_newline() {
    lolr()
        .args(["--force", "--truecolor", "--seed", "1"])
        .write_stdin("x")
        .assert()
        .success()
        .stdout(predicate::str::ends_with("x\x1b[39m"));
}

#[test]
fn no_color_mode_is_byte_transparent() {
    let input = vec![0xff, 0xfe, b'b', b'i', b'n', b'a', b'r', b'y', 0x00];
    lolr()
        .write_stdin(input.clone())
        .assert()
        .success()
        .stdout(input);
}

#[test]
fn deterministic_truecolor_output_matches_ruby_lolcat() {
    lolr()
        .args(["--force", "--truecolor", "--seed", "1"])
        .write_stdin("A\n")
        .assert()
        .success()
        .stdout("\x1b[38;2;153;223;7mA\x1b[39m\n");
}

#[test]
fn deterministic_256_output_matches_ruby_lolcat() {
    lolr()
        .env_remove("COLORTERM")
        .args(["--force", "--seed", "1"])
        .write_stdin("A\n")
        .assert()
        .success()
        .stdout("\x1b[38;5;154mA\x1b[39m\n");
}

#[test]
fn dash_reads_stdin_in_file_order() {
    let mut first = NamedTempFile::new().unwrap();
    let mut last = NamedTempFile::new().unwrap();
    first.write_all(b"first\n").unwrap();
    last.write_all(b"last\n").unwrap();

    lolr()
        .args([
            first.path().to_str().unwrap(),
            "-",
            last.path().to_str().unwrap(),
        ])
        .write_stdin("middle\n")
        .assert()
        .success()
        .stdout("first\nmiddle\nlast\n");
}
