//! Load named environment profiles from a TOML config into a nested shell.

pub mod configfile;
pub mod profile;

pub use configfile::{ConfigFile, NEW_CONFIG_TEMPLATE};
pub use profile::{list_profiles, profile_env, toml_value_to_env};
