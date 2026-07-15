use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn envish() -> Command {
    Command::new(env!("CARGO_BIN_EXE_envish"))
}

fn write_config(contents: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, contents).unwrap();
    (dir, path)
}

#[test]
fn help_succeeds() {
    let output = envish().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Load named environment profiles"));
    assert!(stdout.contains("load"));
    assert!(stdout.contains("list"));
}

#[test]
fn no_subcommand_exits_nonzero() {
    let output = envish().output().unwrap();
    assert!(!output.status.success());
}

#[test]
fn list_prints_sorted_profiles() {
    let (_dir, config) = write_config(
        r#"
[zeta]
A = "1"
[alpha]
B = "2"
"#,
    );

    let output = envish()
        .args(["-c", config.to_str().unwrap(), "list"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "alpha\nzeta\n");
}

#[test]
fn path_creates_missing_config_with_default() {
    let dir = tempdir().unwrap();
    let config = dir.path().join("config.toml");
    assert!(!config.exists());

    let output = envish()
        .args(["-c", config.to_str().unwrap(), "path"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        config.to_str().unwrap()
    );
    assert_eq!(fs::read_to_string(&config).unwrap(), "[DEFAULT]\n");
}

#[test]
fn path_prints_config_path() {
    let (_dir, config) = write_config("");

    let output = envish()
        .args(["-c", config.to_str().unwrap(), "path"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        config.to_str().unwrap()
    );
}

#[test]
fn load_missing_profile_fails() {
    let (_dir, config) = write_config("[dev]\nFOO = \"bar\"\n");

    let output = envish()
        .args(["-c", config.to_str().unwrap(), "load", "prod"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("profile `prod` not found"), "{stderr}");
}

#[cfg(unix)]
#[test]
fn load_applies_env_without_toml_quotes() {
    let (_dir, config) = write_config(
        r#"
[dev]
API_URL = "https://example.com"
DEBUG = "1"
PORT = 8080
"#,
    );

    let output = envish()
        .args(["-c", config.to_str().unwrap(), "load", "dev"])
        .env("SHELL", "/bin/sh")
        .env_remove("API_URL")
        .env("PORT", "3000")
        .env("DEBUG", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Write a short script for the nested shell.
    let mut child = output;
    use std::io::Write;
    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, r#"printf 'API_URL=%s\n' "$API_URL""#).unwrap();
        writeln!(stdin, r#"printf 'PORT=%s\n' "$PORT""#).unwrap();
        writeln!(stdin, "exit 0").unwrap();
    }

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "API_URL=https://example.com\nPORT=8080\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Setting environment variables from profile `dev`"),
        "{stderr}"
    );
    assert!(stderr.contains("all: API_URL, DEBUG, PORT"), "{stderr}");
    assert!(stderr.contains("overwritten: PORT"), "{stderr}");
    assert!(stderr.contains("unchanged: DEBUG"), "{stderr}");
    assert!(!stderr.contains('='), "{stderr}");
}

#[cfg(unix)]
#[test]
fn edit_writes_valid_toml_from_editor() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let config = dir.path().join("config.toml");
    fs::write(&config, "[old]\nA = \"1\"\n").unwrap();

    let editor = dir.path().join("fake-editor.sh");
    fs::write(
        &editor,
        r#"#!/bin/sh
cat > "$1" <<'EOF'
[new]
B = "2"
EOF
"#,
    )
    .unwrap();
    let mut perms = fs::metadata(&editor).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&editor, perms).unwrap();

    let output = envish()
        .args(["-c", config.to_str().unwrap(), "edit"])
        .env("VISUAL", &editor)
        .env_remove("EDITOR")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(fs::read_to_string(config).unwrap(), "[new]\nB = \"2\"\n");
}

#[cfg(unix)]
#[test]
fn edit_rejects_invalid_toml() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let config = dir.path().join("config.toml");
    let original = "[keep]\nA = \"1\"\n";
    fs::write(&config, original).unwrap();

    let editor = dir.path().join("fake-editor.sh");
    fs::write(
        &editor,
        r#"#!/bin/sh
printf 'this is not toml [[' > "$1"
"#,
    )
    .unwrap();
    let mut perms = fs::metadata(&editor).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&editor, perms).unwrap();

    let output = envish()
        .args(["-c", config.to_str().unwrap(), "edit"])
        .env("VISUAL", &editor)
        .env_remove("EDITOR")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not valid TOML"), "{stderr}");
    assert_eq!(fs::read_to_string(config).unwrap(), original);
}
