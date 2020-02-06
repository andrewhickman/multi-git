use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use failure::{Error, ResultExt};
use serde::Deserialize;

pub const CONFIG_PATH: &str = "MULTIGIT_CONFIG_PATH";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub root: PathBuf,
    pub editor: Option<String>,
    #[serde(default)]
    pub aliases: HashMap<String, PathBuf>,
}

pub fn parse() -> Result<Config, Error> {
    match env::var_os(CONFIG_PATH) {
        Some(path) => parse_file(path),
        None => Config::default(),
    }
}

fn parse_file(path: impl AsRef<Path> + Into<PathBuf>) -> Result<Config, Error> {
    log::debug!("Reading config from `{}`", path.as_ref().display());

    let reader = fs_err::read_to_string(path)?;
    let config: Config = toml::from_str(&reader).context("failed to parse TOML")?;

    Ok(config)
}

impl Config {
    fn default() -> Result<Config, Error> {
        Ok(Config {
            root: env::current_dir().context("failed to get current directory")?,
            editor: None,
            aliases: HashMap::new(),
        })
    }
}
