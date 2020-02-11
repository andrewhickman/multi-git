use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fmt};

use failure::{Error, ResultExt};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{de, Deserialize, Deserializer};

pub const FILE_PATH_VAR: &str = "MULTIGIT_CONFIG_PATH";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    pub root: PathBuf,
    #[serde(flatten)]
    pub default_settings: Settings,
    #[serde(default)]
    pub aliases: BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub settings: SettingsMatcher,
}

pub fn parse() -> Result<Config, Error> {
    match file_path() {
        Some(path) => parse_file(path),
        None => Config::default(),
    }
}

pub fn file_path() -> Option<PathBuf> {
    env::var_os(FILE_PATH_VAR).map(PathBuf::from)
}

fn parse_file(path: PathBuf) -> Result<Config, Error> {
    log::debug!("Reading config from `{}`", path.display());

    let reader = fs_err::read_to_string(path)?;
    let config: Config = toml::from_str(&reader).context("failed to parse TOML")?;

    Ok(config)
}

impl Config {
    pub fn settings<P>(&self, path: P) -> Settings
    where
        P: AsRef<Path>,
    {
        let mut result = self.default_settings.clone();
        self.settings.get(&mut result, path.as_ref());
        log::debug!(
            "got merged settings for path `{}`: {:?}",
            path.as_ref().display(),
            result
        );
        result
    }

    pub fn get_relative_path<'a>(&self, path: &'a Path) -> &'a Path {
        path.strip_prefix(&self.root).unwrap_or(path)
    }

    fn default() -> Result<Config, Error> {
        Ok(Config {
            root: env::current_dir().context("failed to get current directory")?,
            aliases: BTreeMap::new(),
            settings: SettingsMatcher::default(),
            default_settings: Settings::default(),
        })
    }
}

pub struct SettingsMatcher {
    globs: GlobSet,
    settings: Vec<Settings>,
}

impl SettingsMatcher {
    fn get(&self, base: &mut Settings, path: &Path) {
        for idx in self.globs.matches(path) {
            log::trace!(
                "found settings for path `{}`: {:?}",
                path.display(),
                self.settings[idx]
            );
            base.merge(&self.settings[idx]);
        }
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Settings {
    pub default_branch: Option<String>,
    pub editor: Option<String>,
    pub ignore: Option<bool>,
}

impl Settings {
    fn merge(&mut self, other: &Self) {
        if other.default_branch.is_some() {
            self.default_branch.clone_from(&other.default_branch);
        }
        if other.editor.is_some() {
            self.editor.clone_from(&other.editor);
        }
        if other.ignore.is_some() {
            self.ignore.clone_from(&other.ignore);
        }
    }
}

impl Default for SettingsMatcher {
    fn default() -> Self {
        SettingsMatcher {
            globs: GlobSet::empty(),
            settings: Vec::new(),
        }
    }
}

impl<'de> Deserialize<'de> for SettingsMatcher {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = SettingsMatcher;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut settings = Vec::with_capacity(map.size_hint().unwrap_or(4));
                let mut globs = GlobSetBuilder::new();

                while let Some((glob, entry)) = map.next_entry::<Cow<str>, Settings>()? {
                    globs.add(Glob::new(&glob).map_err(de::Error::custom)?);
                    settings.push(entry);
                }

                Ok(SettingsMatcher {
                    settings,
                    globs: globs.build().map_err(de::Error::custom)?,
                })
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

impl fmt::Debug for SettingsMatcher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SettingsMatcher")
            .field("globs", &"GlobSet")
            .field("settings", &self.settings)
            .finish()
    }
}
