use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fmt};

use fn_error_context::context;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{de, Deserialize, Deserializer};
use toml_edit::Document;

pub const FILE_PATH_VAR: &str = "MULTIGIT_CONFIG_PATH";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    pub root: PathBuf,
    #[serde(default)]
    pub default_shell: Shell,
    #[serde(flatten)]
    pub default_settings: Settings,
    #[serde(default)]
    pub aliases: BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub settings: SettingsMatcher,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shell {
    None,
    #[serde(alias = "sh")]
    Bash,
    Cmd,
    Powershell,
    #[serde(alias = "pwsh")]
    PowershellCore,
}

pub fn parse() -> crate::Result<Config> {
    match file_path() {
        Some(path) => {
            let config = parse_file(path)?;
            config
                .validate()
                .map_err(|err| crate::Error::with_context(err, "invalid config"))?;
            Ok(config)
        }
        None => Config::default(),
    }
}

#[context("failed to edit config file")]
pub fn edit(f: impl FnOnce(&mut Document) -> crate::Result<()>) -> crate::Result<()> {
    let path = expect_file_path()?;
    log::debug!("Reading config from `{}`", path.display());

    let file = fs_err::read_to_string(&path)?;
    let mut document = file.parse::<Document>()?;

    f(&mut document)?;

    log::debug!("Writing config to `{}`", path.display());
    fs_err::write(path, document.to_string_in_original_order())?;
    Ok(())
}

pub fn expect_file_path() -> crate::Result<PathBuf> {
    file_path().ok_or_else(|| {
        crate::Error::from_message(format!(
            "the `{}` environment variable must be set",
            FILE_PATH_VAR
        ))
    })
}

pub fn file_path() -> Option<PathBuf> {
    env::var_os(FILE_PATH_VAR).map(PathBuf::from)
}

fn parse_file(path: PathBuf) -> crate::Result<Config> {
    log::debug!("Reading config from `{}`", path.display());

    let reader = fs_err::read_to_string(path)?;
    let config: Config = toml::from_str(&reader)
        .map_err(|err| crate::Error::with_context(err, "failed to parse TOML"))?;

    Ok(config)
}

impl Config {
    pub fn settings<P>(&self, relative_path: P) -> Settings
    where
        P: AsRef<Path>,
    {
        let mut result = self.default_settings.clone();
        self.settings.get(&mut result, relative_path.as_ref());
        log::debug!(
            "got merged settings for path `{}`: {:?}",
            relative_path.as_ref().display(),
            result
        );
        result
    }

    pub fn get_relative_path<'a>(&self, path: &'a Path) -> &'a Path {
        path.strip_prefix(&self.root).unwrap_or(path)
    }

    fn default() -> crate::Result<Config> {
        Ok(Config {
            root: env::current_dir().map_err(|err| {
                crate::Error::with_context(err, "failed to get current directory")
            })?,
            default_shell: Shell::default(),
            aliases: BTreeMap::new(),
            settings: SettingsMatcher::default(),
            default_settings: Settings::default(),
        })
    }

    fn validate(&self) -> crate::Result<()> {
        if !self.root.exists() {
            return Err(crate::Error::from_message(format!(
                "root path `{}` is invalid",
                self.root.display()
            )));
        }

        Ok(())
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
    pub default_remote: Option<String>,
    pub ssh: Option<SshSettings>,
    pub editor: Option<String>,
    pub ignore: Option<bool>,
    pub prune: Option<bool>,
    #[serde(skip)]
    glob: String,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct SshSettings {
    pub passphrase: Option<String>,
    pub public_key_path: Option<PathBuf>,
    pub private_key_path: PathBuf,
}

impl Settings {
    fn merge(&mut self, other: &Self) {
        if other.default_branch.is_some() {
            self.default_branch.clone_from(&other.default_branch);
        }
        if other.default_remote.is_some() {
            self.default_remote.clone_from(&other.default_remote);
        }
        if other.ssh.is_some() {
            self.ssh.clone_from(&other.ssh);
        }
        if other.editor.is_some() {
            self.editor.clone_from(&other.editor);
        }
        if other.ignore.is_some() {
            self.ignore.clone_from(&other.ignore);
        }
        if other.prune.is_some() {
            self.prune.clone_from(&other.prune);
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

                while let Some((glob, entry)) = map.next_entry::<String, Settings>()? {
                    globs.add(Glob::new(&glob).map_err(de::Error::custom)?);
                    settings.push(Settings { glob, ..entry });
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
