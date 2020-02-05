use std::path::{Path, PathBuf};
use std::{env, fmt};

use failure::{Error, ResultExt};
use serde::{de, Deserialize, Deserializer};

pub const CONFIG_PATH: &str = "MULTIGIT_CONFIG_PATH";

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_path")]
    root: PathBuf,
    aliases: Vec<Alias>,
}

#[derive(Debug, Deserialize)]
pub struct Alias {
    name: String,
    path: String,
}

pub fn parse() -> Result<Config, Error> {
    Ok(match env::var_os(CONFIG_PATH) {
        Some(path) => parse_file(path).context("failed to read config file")?,
        None => Config::default()?,
    })
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
            aliases: Vec::new(),
        })
    }
}

fn deserialize_path<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    struct PathVisitor;

    impl<'de> de::Visitor<'de> for PathVisitor {
        type Value = PathBuf;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a path")
        }

        fn visit_string<E>(self, s: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(s.into())
        }
    }

    deserializer.deserialize_string(PathVisitor)
}
