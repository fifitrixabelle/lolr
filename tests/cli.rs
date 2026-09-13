use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

struct LolrCommand {
    command: Command,
    _config_home: TempDir,
}

impl Deref for LolrCommand {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}

impl DerefMut for LolrCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.command
    }
}

fn lolr() -> LolrCommand {
    let config_home = tempfile::tempdir().unwrap();
    let command = lolr_with_config_home(config_home.path());
    LolrCommand {
        command,
        _config_home: config_home,
    }
}

fn lolr_with_config_home(config_home: &Path) -> Command {
    let mut command = Command::cargo_bin("lolr").unwrap();
    command.env("XDG_CONFIG_HOME", config_home);
    command
}

fn write_config(config_home: &Path, contents: &str) {
    let config_dir = config_home.join("lolr");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), contents).unwrap();
}

#[test]
fn help_works() {
    lolr()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rainbow colorizer"))
        .stdout(predicate::str::contains(
            "available: rainbow, fire, ocean, pastel, neon, sunset, forest, synthwave, viridis, aura",
        ));
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
fn lists_available_gradients() {
    let config_home = tempfile::tempdir().unwrap();
    lolr_with_config_home(config_home.path())
        .arg("--list-gradients")
        .assert()
        .success()
        .stdout("rainbow\nfire\nocean\npastel\nneon\nsunset\nforest\nsynthwave\nviridis\naura\n");
    assert!(!config_home.path().join("lolr/config.toml").exists());
}

#[test]
fn creates_default_xdg_config_on_first_run() {
    let config_home = tempfile::tempdir().unwrap();

    lolr_with_config_home(config_home.path())
        .write_stdin("test")
        .assert()
        .success();

    let config = fs::read_to_string(config_home.path().join("lolr/config.toml")).unwrap();
    assert!(config.contains("spread = 3.0"));
    assert!(config.contains("gradient = \"rainbow\""));
    assert!(config.contains("force = false"));
}

#[test]
fn config_values_apply_and_cli_values_take_precedence() {
    let config_home = tempfile::tempdir().unwrap();
    write_config(
        config_home.path(),
        "force = true\ntruecolor = true\nseed = 1\ngradient = \"fire\"\n",
    );

    lolr_with_config_home(config_home.path())
        .args(["--gradient", "rainbow"])
        .write_stdin("A\n")
        .assert()
        .success()
        .stdout("\x1b[38;2;153;223;7mA\x1b[39m\n");
}

#[test]
fn rejects_invalid_config_values() {
    let config_home = tempfile::tempdir().unwrap();
    write_config(config_home.path(), "duration = 0\n");

    lolr_with_config_home(config_home.path())
        .write_stdin("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid config value for duration",
        ));
}

#[test]
fn rejects_unknown_config_keys_to_catch_typos() {
    let config_home = tempfile::tempdir().unwrap();
    write_config(config_home.path(), "gradent = \"aura\"\n");

    lolr_with_config_home(config_home.path())
        .write_stdin("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown field `gradent`"));
}

#[test]
fn no_config_bypasses_a_broken_config() {
    let config_home = tempfile::tempdir().unwrap();
    write_config(config_home.path(), "this is not toml");

    lolr_with_config_home(config_home.path())
        .arg("--no-config")
        .write_stdin("test")
        .assert()
        .success()
        .stdout("test");
}

#[test]
fn negative_flags_override_configured_booleans() {
    let config_home = tempfile::tempdir().unwrap();
    write_config(
        config_home.path(),
        "force = true\ntruecolor = true\nseed = 1\n",
    );

    lolr_with_config_home(config_home.path())
        .arg("--no-force")
        .write_stdin("test")
        .assert()
        .success()
        .stdout("test");

    lolr_with_config_home(config_home.path())
        .env_remove("COLORTERM")
        .arg("--no-truecolor")
        .write_stdin("A\n")
        .assert()
        .success()
        .stdout("\x1b[38;5;154mA\x1b[39m\n");
}

#[test]
fn supports_custom_and_environment_config_paths() {
    let directory = tempfile::tempdir().unwrap();
    let cli_config = directory.path().join("cli.toml");
    let env_config = directory.path().join("env.toml");
    fs::write(&cli_config, "force = true\n").unwrap();
    fs::write(&env_config, "force = false\n").unwrap();

    lolr()
        .env("LOLR_CONFIG", &env_config)
        .args(["--config", cli_config.to_str().unwrap()])
        .write_stdin("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b["));

    lolr()
        .env("LOLR_CONFIG", &env_config)
        .arg("--print-config-path")
        .assert()
        .success()
        .stdout(format!("{}\n", env_config.display()));
}

#[test]
fn never_overwrites_an_existing_config() {
    let config_home = tempfile::tempdir().unwrap();
    let original = "# keep this comment\nforce = false\n";
    write_config(config_home.path(), original);

    lolr_with_config_home(config_home.path())
        .write_stdin("test")
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(config_home.path().join("lolr/config.toml")).unwrap(),
        original
    );
}

#[test]
fn rejects_non_finite_frequency_from_cli() {
    lolr().args(["--freq", "NaN"]).assert().failure();
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
