use std::path::PathBuf;

use failure::{ensure, Error};

use crate::config::Config;

pub fn resolve(name: &str, config: &Config) -> Result<PathBuf, Error> {
    let full_path = if let Some(path) = config.aliases.get(name) {
        let full_path = config.root.join(path);
        log::trace!("resolved alias `{}` to `{}`", name, full_path.display());
        full_path
    } else {
        let full_path = config.root.join(name);
        log::trace!("resolved path `{}` to `{}`", name, full_path.display());
        full_path
    };

    ensure!(
        full_path.exists(),
        "path `{}` does not exist",
        full_path.display()
    );
    Ok(full_path)
}
