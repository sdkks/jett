use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::ui::theme::{Theme, ThemeMode};

mod colors;
#[cfg(test)]
mod tests;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    theme: Option<ThemeConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            theme: None,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ThemeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<Mode>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    custom_schemes: BTreeMap<String, BTreeMap<String, toml::Value>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Dark,
    Light,
}

impl From<Mode> for ThemeMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Dark => Self::Dark,
            Mode::Light => Self::Light,
        }
    }
}

pub fn path() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        windows_path(env::var_os("APPDATA"))
    }
    #[cfg(not(windows))]
    {
        unix_path(env::var_os("XDG_CONFIG_HOME"), env::var_os("HOME"))
    }
}

#[cfg(any(not(windows), test))]
fn unix_path(xdg: Option<OsString>, home: Option<OsString>) -> Result<PathBuf> {
    let base = match xdg.filter(|value| !value.is_empty()) {
        Some(xdg) => PathBuf::from(xdg),
        None => PathBuf::from(
            home.filter(|value| !value.is_empty())
                .context("neither XDG_CONFIG_HOME nor HOME is set")?,
        )
        .join(".config"),
    };
    Ok(base.join("jett/config.toml"))
}

#[cfg(any(windows, test))]
fn windows_path(appdata: Option<OsString>) -> Result<PathBuf> {
    Ok(PathBuf::from(
        appdata
            .filter(|value| !value.is_empty())
            .context("APPDATA is not set")?,
    )
    .join("jett")
    .join("config.toml"))
}

impl Config {
    // Reading and validation are separate so `config set` can repair an unknown selection.
    fn read(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text)
                .with_context(|| format!("could not parse config {}", path.display())),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => {
                Err(error).with_context(|| format!("could not read config {}", path.display()))
            }
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let config = Self::read(path)?;
        config
            .validate()
            .with_context(|| format!("invalid config {}", path.display()))?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.version != 1 {
            bail!(
                "unsupported config version {}; expected version = 1",
                self.version
            );
        }
        if let Some(theme) = &self.theme {
            for name in theme.custom_schemes.keys() {
                if name.is_empty() || name.chars().any(char::is_control) {
                    bail!("custom scheme names must be nonempty and contain no control characters");
                }
                if Theme::named(name).is_some() {
                    bail!("custom scheme '{name}' conflicts with a built-in scheme");
                }
                self.resolve(name)?;
            }
            if let Some(name) = &theme.scheme {
                self.resolve(name)?;
            }
        }
        Ok(())
    }

    pub fn resolve(&self, name: &str) -> Result<Theme> {
        let config = self.theme.as_ref();
        let mode = config.and_then(|theme| theme.mode);
        if let Some(mut theme) = Theme::named(name) {
            if let Some(mode) = mode {
                theme.mode = mode.into();
            }
            return Ok(theme);
        }
        let colors = config
            .and_then(|theme| theme.custom_schemes.get(name))
            .with_context(|| format!("unknown theme scheme '{name}'"))?;
        let custom_mode = colors
            .get("mode")
            .map(|value| match value.as_str() {
                Some("dark") => Ok(Mode::Dark),
                Some("light") => Ok(Mode::Light),
                _ => bail!("theme.custom_schemes.{name}.mode: expected 'dark' or 'light'"),
            })
            .transpose()?;
        let mode = mode.or(custom_mode).unwrap_or_else(|| {
            let name = name.to_ascii_lowercase();
            if name.contains("light") || name.contains("latte") {
                Mode::Light
            } else {
                Mode::Dark
            }
        });
        let mut theme = Theme::named(match mode {
            Mode::Dark => "dark",
            Mode::Light => "light",
        })
        .expect("dark and light are built-in themes");
        for (role, value) in colors {
            if role == "mode" {
                continue;
            }
            colors::apply(&mut theme, role, value)
                .with_context(|| format!("theme.custom_schemes.{name}.{role}"))?;
        }
        Ok(theme)
    }

    pub fn scheme_names(&self) -> Vec<String> {
        let mut names: Vec<String> = crate::ui::theme::BUILTIN_SCHEMES
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        if let Some(theme) = &self.theme {
            names.extend(theme.custom_schemes.keys().cloned());
        }
        names
    }

    pub fn set_scheme(path: &Path, name: &str) -> Result<()> {
        let mut config = Self::read(path)?;
        config.theme.get_or_insert_with(ThemeConfig::default).scheme = Some(name.to_owned());
        config.validate()?;
        let text = toml::to_string_pretty(&config).context("could not serialize config")?;
        let parent = path
            .parent()
            .context("config path has no parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create config directory {}", parent.display()))?;
        static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);
        let (temporary, mut file) = loop {
            let temporary = parent.join(format!(
                ".jett-config-{}-{}.tmp",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(file) => break (temporary, file),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("could not write config {}", path.display()));
                }
            }
        };
        // Never truncate the existing config before its replacement is complete.
        let result = (|| {
            match fs::metadata(path) {
                Ok(metadata) => file.set_permissions(metadata.permissions())?,
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.with_context(|| format!("could not write config {}", path.display()))
    }
}

pub struct ThemeSelection {
    pub theme: Theme,
    pub name: String,
    pub config: Config,
    pub notice: Option<String>,
}

pub fn select_theme(flag: Option<&str>, config: Result<Config>) -> Result<ThemeSelection> {
    let (config, notice) = match config {
        Ok(config) => (config, None),
        Err(error) => (
            Config::default(),
            Some(format!(
                "Config ignored: {error:#}. Run jett config validate for details."
            )),
        ),
    };
    let name = flag
        .or_else(|| {
            config
                .theme
                .as_ref()
                .and_then(|theme| theme.scheme.as_deref())
        })
        .unwrap_or("default");
    Ok(ThemeSelection {
        theme: config.resolve(name)?,
        name: name.to_owned(),
        config,
        notice,
    })
}
