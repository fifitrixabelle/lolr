use assert_cmd::Command;
use predicates::prelude::*;

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
