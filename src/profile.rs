use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use toml::Table;
use toml::Value;

/// Profile name that is always applied before the requested profile.
pub const DEFAULT_PROFILE: &str = "DEFAULT";

/// Convert a TOML value into a string suitable for an environment variable.
pub fn toml_value_to_env(value: &Value) -> Result<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Integer(i) => Ok(i.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Boolean(b) => Ok(b.to_string()),
        Value::Datetime(dt) => Ok(dt.to_string()),
        Value::Array(_) | Value::Table(_) => {
            bail!("unsupported TOML type for environment variables (arrays and tables)")
        }
    }
}

fn table_to_env(profile_name: &str, profile: &Table) -> Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for (key, value) in profile {
        let converted = toml_value_to_env(value)
            .with_context(|| format!("invalid value for `{profile_name}.{key}`"))?;
        env.insert(key.clone(), converted);
    }
    Ok(env)
}

fn merge_env(into: &mut BTreeMap<String, String>, from: BTreeMap<String, String>) {
    for (key, value) in from {
        into.insert(key, value);
    }
}

/// Parse config text and return environment variables for the named profile.
///
/// If a `[DEFAULT]` table exists, its variables are applied first. The named
/// profile is then applied on top (overriding defaults on key conflicts).
pub fn profile_env(config: &str, name: &str) -> Result<BTreeMap<String, String>> {
    let root: Value = toml::from_str(config).context("failed to parse config as TOML")?;
    let table = root
        .as_table()
        .context("config root must be a TOML table")?;

    let mut env = BTreeMap::new();

    if let Some(default) = table.get(DEFAULT_PROFILE) {
        let default = default
            .as_table()
            .with_context(|| format!("profile `{DEFAULT_PROFILE}` must be a TOML table"))?;
        merge_env(&mut env, table_to_env(DEFAULT_PROFILE, default)?);
    }

    if name == DEFAULT_PROFILE {
        if !table.contains_key(DEFAULT_PROFILE) {
            bail!("profile `{DEFAULT_PROFILE}` not found");
        }
        return Ok(env);
    }

    let profile = table
        .get(name)
        .with_context(|| format!("profile `{name}` not found"))?;
    let profile = profile
        .as_table()
        .with_context(|| format!("profile `{name}` must be a TOML table"))?;
    merge_env(&mut env, table_to_env(name, profile)?);

    Ok(env)
}

/// List top-level profile names (TOML tables) in the config.
///
/// `[DEFAULT]` is omitted — it is always applied when loading another profile.
pub fn list_profiles(config: &str) -> Result<Vec<String>> {
    if config.trim().is_empty() {
        return Ok(Vec::new());
    }

    let root: Value = toml::from_str(config).context("failed to parse config as TOML")?;
    let table = root
        .as_table()
        .context("config root must be a TOML table")?;

    let mut names: Vec<String> = table
        .iter()
        .filter(|(name, value)| *name != DEFAULT_PROFILE && value.is_table())
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_values_do_not_include_toml_quotes() {
        let env = profile_env(
            r#"
[dev]
API_URL = "https://example.com"
FLAG = true
PORT = 8080
"#,
            "dev",
        )
        .unwrap();

        assert_eq!(env.get("API_URL").unwrap(), "https://example.com");
        assert_eq!(env.get("FLAG").unwrap(), "true");
        assert_eq!(env.get("PORT").unwrap(), "8080");
    }

    #[test]
    fn missing_profile_errors() {
        let err = profile_env("[dev]\nFOO = \"bar\"\n", "prod").unwrap_err();
        assert!(err.to_string().contains("profile `prod` not found"));
    }

    #[test]
    fn nested_tables_are_rejected() {
        let err = profile_env(
            r#"
[dev]
nested = { a = 1 }
"#,
            "dev",
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid value for `dev.nested`"));
    }

    #[test]
    fn list_profiles_sorted() {
        let names = list_profiles(
            r#"
[zeta]
A = "1"
[alpha]
B = "2"
"#,
        )
        .unwrap();
        assert_eq!(names, vec!["alpha".to_string(), "zeta".to_string()]);
    }

    #[test]
    fn empty_config_lists_nothing() {
        assert!(list_profiles("").unwrap().is_empty());
    }

    #[test]
    fn default_is_applied_before_named_profile() {
        let env = profile_env(
            r#"
[DEFAULT]
SHARED = "from-default"
OVERRIDE = "default"

[dev]
OVERRIDE = "dev"
DEV_ONLY = "1"
"#,
            "dev",
        )
        .unwrap();

        assert_eq!(env.get("SHARED").unwrap(), "from-default");
        assert_eq!(env.get("OVERRIDE").unwrap(), "dev");
        assert_eq!(env.get("DEV_ONLY").unwrap(), "1");
    }

    #[test]
    fn list_profiles_excludes_default() {
        let names = list_profiles(
            r#"
[DEFAULT]
X = "1"
[dev]
Y = "2"
"#,
        )
        .unwrap();
        assert_eq!(names, vec!["dev".to_string()]);
    }

    #[test]
    fn load_default_alone() {
        let env = profile_env(
            r#"
[DEFAULT]
X = "1"
[dev]
Y = "2"
"#,
            "DEFAULT",
        )
        .unwrap();
        assert_eq!(env.get("X").unwrap(), "1");
        assert!(!env.contains_key("Y"));
    }
}
