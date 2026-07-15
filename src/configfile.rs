use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

/// Contents written when a new config file is created.
pub const NEW_CONFIG_TEMPLATE: &str = "[DEFAULT]\n";

pub struct ConfigFile {
    path: PathBuf,
}

impl ConfigFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create the config file with a `[DEFAULT]` section if it does not exist yet.
    pub fn ensure_exists(&self) -> Result<()> {
        match fs::metadata(&self.path) {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => self.write(NEW_CONFIG_TEMPLATE),
            Err(err) => Err(err)
                .with_context(|| format!("failed to access config file {}", self.path.display())),
        }
    }

    /// Read the config file. A missing file is treated as empty content.
    pub fn read(&self) -> Result<String> {
        match fs::read_to_string(&self.path) {
            Ok(content) => Ok(content),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(String::new()),
            Err(err) => Err(err)
                .with_context(|| format!("failed to read config file {}", self.path.display())),
        }
    }

    pub fn write(&self, content: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }

        let mut file = fs::File::create(&self.path)
            .with_context(|| format!("failed to write config file {}", self.path.display()))?;
        file.write_all(content.as_bytes())
            .with_context(|| format!("failed to write config file {}", self.path.display()))?;
        Ok(())
    }
}
