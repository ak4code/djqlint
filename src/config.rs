//! `.djqlint.toml` configuration.
//!
//! ```toml
//! [rules]
//! disabled = ["P002"]          # turn rules off by code
//!
//! [files]
//! exclude = ["migrations/", "tests/", "/snapshots/"]
//! respect_gitignore = true     # honour .gitignore (default: true)
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub rules: RulesConfig,
    #[serde(default)]
    pub files: FilesConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesConfig {
    /// Rule codes to disable entirely, e.g. `["P002", "N001"]`.
    #[serde(default)]
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilesConfig {
    /// Substrings; any path containing one is skipped.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Whether to honour `.gitignore`/`.ignore` files.
    #[serde(default = "default_true")]
    pub respect_gitignore: bool,
}

impl Default for FilesConfig {
    fn default() -> Self {
        FilesConfig {
            exclude: Vec::new(),
            respect_gitignore: true,
        }
    }
}

fn default_true() -> bool {
    true
}

const DEFAULT_FILENAME: &str = ".djqlint.toml";

impl Config {
    /// Load configuration.
    ///
    /// * `explicit` — an explicit `--config` path (errors if unreadable/invalid).
    /// * otherwise — auto-discover `.djqlint.toml` in the current directory;
    ///   absence is not an error and yields [`Config::default`].
    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        match explicit {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .with_context(|| format!("reading config file {}", path.display()))?;
                toml::from_str(&text)
                    .with_context(|| format!("parsing config file {}", path.display()))
            }
            None => {
                let default = PathBuf::from(DEFAULT_FILENAME);
                if default.exists() {
                    let text = std::fs::read_to_string(&default)
                        .with_context(|| format!("reading {}", default.display()))?;
                    toml::from_str(&text).with_context(|| format!("parsing {}", default.display()))
                } else {
                    Ok(Config::default())
                }
            }
        }
    }
}
