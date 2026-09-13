use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use lolr::Gradient;

pub const DEFAULT_SPREAD: f64 = 3.0;
pub const DEFAULT_FREQ: f64 = 0.1;
pub const DEFAULT_SEED: u64 = 0;
pub const DEFAULT_ANIMATE: bool = false;
pub const DEFAULT_DURATION: u32 = 6;
pub const DEFAULT_SPEED: f64 = 40.0;
pub const DEFAULT_INVERT: bool = false;
pub const DEFAULT_TRUECOLOR: bool = false;
pub const DEFAULT_FORCE: bool = false;

const MAX_CONFIG_SIZE: u64 = 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub spread: f64,
    pub freq: f64,
    pub seed: u64,
    pub animate: bool,
    pub duration: u32,
    pub speed: f64,
    pub invert: bool,
    pub truecolor: bool,
    pub force: bool,
    pub gradient: Gradient,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spread: DEFAULT_SPREAD,
            freq: DEFAULT_FREQ,
            seed: DEFAULT_SEED,
            animate: DEFAULT_ANIMATE,
            duration: DEFAULT_DURATION,
            speed: DEFAULT_SPEED,
            invert: DEFAULT_INVERT,
            truecolor: DEFAULT_TRUECOLOR,
            force: DEFAULT_FORCE,
            gradient: Gradient::default(),
        }
    }
}

impl Config {
    pub fn resolve_path(cli_path: Option<&Path>) -> io::Result<PathBuf> {
        if let Some(path) = cli_path {
            return Ok(path.to_owned());
        }

        if let Some(path) = nonempty_env_path("LOLR_CONFIG") {
            return Ok(path);
        }

        if let Some(path) = nonempty_env_path("XDG_CONFIG_HOME") {
            if path.is_absolute() {
                return Ok(path.join("lolr/config.toml"));
            }
        }

        nonempty_env_path("HOME")
            .map(|home| home.join(".config/lolr/config.toml"))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "cannot locate config directory: set HOME, XDG_CONFIG_HOME, or LOLR_CONFIG",
                )
            })
    }

    pub fn load_or_create(path: &Path) -> io::Result<Self> {
        if !path.exists() {
            create_default_config(path)?;
        }

        let metadata =
            fs::metadata(path).map_err(|error| config_io_error("inspect", path, error))?;
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("config path is not a file: {}", path.display()),
            ));
        }
        if metadata.len() > MAX_CONFIG_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("config file is larger than 1 MiB: {}", path.display()),
            ));
        }

        let contents =
            fs::read_to_string(path).map_err(|error| config_io_error("read", path, error))?;
        let config: Self = toml::from_str(&contents).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse {}: {error}", path.display()),
            )
        })?;
        config.validate(path)?;
        Ok(config)
    }

    fn validate(&self, path: &Path) -> io::Result<()> {
        validate_minimum(self.spread, "spread", path)?;
        if !self.freq.is_finite() {
            return Err(invalid_value("freq", "must be finite", path));
        }
        if self.duration == 0 {
            return Err(invalid_value("duration", "must be >= 1", path));
        }
        validate_minimum(self.speed, "speed", path)?;
        Ok(())
    }
}

fn nonempty_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn create_default_config(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| config_io_error("create directory for", path, error))?;

    let serialized = toml::to_string_pretty(&Config::default()).map_err(|error| {
        io::Error::other(format!("failed to serialize default config: {error}"))
    })?;
    let contents = format!("# lolr configuration\n{serialized}");

    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| config_io_error("create temporary file for", path, error))?;
    temporary
        .write_all(contents.as_bytes())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| config_io_error("write", path, error))?;

    match temporary.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(config_io_error("install", path, error.error)),
    }
}

fn validate_minimum(value: f64, name: &str, path: &Path) -> io::Result<()> {
    if value.is_finite() && value >= 0.1 {
        Ok(())
    } else {
        Err(invalid_value(name, "must be a finite number >= 0.1", path))
    }
}

fn invalid_value(name: &str, message: &str, path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "invalid config value for {name} in {}: {message}",
            path.display()
        ),
    )
}

fn config_io_error(action: &str, path: &Path, error: io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!("failed to {action} config {}: {error}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    #[test]
    fn concurrent_first_loads_create_one_complete_config() {
        let directory = tempfile::tempdir().unwrap();
        let path = Arc::new(directory.path().join("nested/config.toml"));
        let barrier = Arc::new(Barrier::new(8));

        let workers: Vec<_> = (0..8)
            .map(|_| {
                let path = Arc::clone(&path);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    Config::load_or_create(&path).unwrap()
                })
            })
            .collect();

        for worker in workers {
            let config = worker.join().unwrap();
            assert_eq!(config.gradient, Gradient::Rainbow);
        }

        let contents = fs::read_to_string(path.as_ref()).unwrap();
        let config: Config = toml::from_str(&contents).unwrap();
        assert_eq!(config.duration, DEFAULT_DURATION);
    }
}
