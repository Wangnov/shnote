use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn shnote_cmd() -> Command {
    cargo_bin_cmd!("shnote")
}

// === Help and version ===
#[test]
fn test_help() {
    shnote_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_version() {
    shnote_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("shnote"));
}

#[test]
fn test_lang_flag_zh() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--lang", "zh", "config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
fn test_lang_flag_en() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--lang", "en", "config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

// === run command ===
#[test]
fn test_run_requires_what_why() {
    shnote_cmd()
        .args(["run", "echo", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--what"));
}

#[test]
fn test_i18n_uses_language_env_when_auto() {
    let temp_dir = TempDir::new().unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("LANGUAGE", "zh:en")
        .env_remove("SHNOTE_LANG")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANG")
        .args(["run", "echo", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("需要"));
}

#[test]
fn test_i18n_falls_back_to_english_when_env_empty_and_unknown() {
    let temp_dir = TempDir::new().unwrap();
    let empty_path = TempDir::new().unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", empty_path.path())
        .env("SHNOTE_LANG", "")
        .env("LANG", "fr")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANGUAGE")
        .args(["run", "echo", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires"));
}

#[test]
fn test_i18n_ignores_invalid_config_language_and_uses_env() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();
    fs::write(
        temp_dir.path().join(".shnote/config.toml"),
        "[i18n]\nlanguage = \"invalid\"\n",
    )
    .unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("SHNOTE_LANG", "zh")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANGUAGE")
        .env_remove("LANG")
        .args(["run", "echo", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("需要"));
}

#[test]
fn test_i18n_uses_language_env_without_colon() {
    let temp_dir = TempDir::new().unwrap();
    let empty_path = TempDir::new().unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", empty_path.path())
        .env_remove("SHNOTE_LANG")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env("LANGUAGE", "en_US.UTF-8")
        .env_remove("LANG")
        .args(["run", "echo", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires"));
}

#[cfg(target_os = "macos")]
#[test]
fn test_i18n_uses_macos_defaults_when_env_missing() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let tools_dir = TempDir::new().unwrap();
    let defaults = tools_dir.path().join("defaults");
    fs::write(&defaults, "#!/bin/sh\necho \"zh_CN\"\nexit 0\n").unwrap();
    fs::set_permissions(&defaults, fs::Permissions::from_mode(0o755)).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", tools_dir.path())
        .env_remove("SHNOTE_LANG")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANGUAGE")
        .env_remove("LANG")
        .args(["run", "echo", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("需要"));
}

#[cfg(target_os = "macos")]
#[test]
fn test_i18n_defaults_missing_falls_back_to_english() {
    let temp_dir = TempDir::new().unwrap();
    let empty_path = TempDir::new().unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", empty_path.path())
        .env_remove("SHNOTE_LANG")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANGUAGE")
        .env_remove("LANG")
        .args(["run", "echo", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires"));
}

#[cfg(target_os = "macos")]
#[test]
fn test_i18n_defaults_failure_status_falls_back_to_english() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let tools_dir = TempDir::new().unwrap();
    let defaults = tools_dir.path().join("defaults");
    fs::write(&defaults, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&defaults, fs::Permissions::from_mode(0o755)).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", tools_dir.path())
        .env_remove("SHNOTE_LANG")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANGUAGE")
        .env_remove("LANG")
        .args(["run", "echo", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires"));
}

#[test]
fn test_run_with_what_why() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--what", "测试", "--why", "验证", "run", "echo", "hello"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WHAT: 测试"))
        .stdout(predicate::str::contains("WHY:  验证"))
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn test_run_missing_only_what() {
    shnote_cmd()
        .args(["--what", "test", "run", "echo"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--why"));
}

#[test]
fn test_run_missing_only_why() {
    shnote_cmd()
        .args(["--why", "test", "run", "echo"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--what"));
}

// === py command ===
#[test]
fn test_py_requires_what_why() {
    shnote_cmd()
        .args(["py", "-c", "print(1)"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--what"));
}

#[test]
fn test_py_requires_source() {
    shnote_cmd()
        .args(["--what", "test", "--why", "test", "py"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("source")
                .or(predicate::str::contains("stdin"))
                .or(predicate::str::contains("code"))
                .or(predicate::str::contains("file")),
        );
}

#[test]
fn test_py_inline_code() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args([
            "--what",
            "测试Python",
            "--why",
            "验证",
            "py",
            "-c",
            "print('hello from python')",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("WHAT: 测试Python"))
        .stdout(predicate::str::contains("hello from python"));
}

#[test]
fn test_py_file() {
    let temp_dir = TempDir::new().unwrap();
    let script = temp_dir.path().join("test.py");
    fs::write(&script, "print('from file')").unwrap();

    shnote_cmd()
        .args([
            "--what",
            "测试",
            "--why",
            "验证",
            "py",
            "-f",
            script.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("from file"));
}

#[cfg(unix)]
#[test]
fn test_py_stdin_reads_from_stdin_and_passes_args() {
    let temp_dir = TempDir::new().unwrap();

    // Point python to /bin/sh to avoid depending on system python.
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "set", "python", "/bin/sh"])
        .assert()
        .success();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args([
            "--what", "test", "--why", "test", "py", "--stdin", "arg0", "arg1",
        ])
        .write_stdin("echo stdin-ok\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("stdin-ok"));
}

// === node command ===
#[test]
fn test_node_requires_what_why() {
    shnote_cmd()
        .args(["node", "-c", "console.log(1)"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--what"));
}

#[test]
fn test_node_inline_code() {
    shnote_cmd()
        .args([
            "--what",
            "测试Node",
            "--why",
            "验证",
            "node",
            "-c",
            "console.log('hello from node')",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from node"));
}

#[test]
fn test_node_requires_source() {
    shnote_cmd()
        .args(["--what", "test", "--why", "test", "node"])
        .assert()
        .failure();
}

// === pip command ===
#[test]
fn test_pip_requires_what_why() {
    shnote_cmd()
        .args(["pip", "--version"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--what"));
}

#[test]
fn test_pip_with_what_why() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--what", "test", "--why", "test", "pip", "--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WHAT: test"))
        .stdout(predicate::str::contains("pip"));
}

// === npm command ===
#[test]
fn test_npm_requires_what_why() {
    shnote_cmd()
        .args(["npm", "--version"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--what"));
}

#[test]
fn test_npm_with_what_why() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--what", "test", "--why", "test", "npm", "--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WHAT: test"));
}

// === npx command ===
#[test]
fn test_npx_requires_what_why() {
    shnote_cmd()
        .args(["npx", "--version"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--what"));
}

#[test]
fn test_npx_with_what_why() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--what", "test", "--why", "test", "npx", "--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WHAT: test"));
}

// === config command ===
#[test]
fn test_config_list() {
    shnote_cmd()
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("python"))
        .stdout(predicate::str::contains("node"));
}

#[cfg(unix)]
#[test]
fn test_config_errors_when_config_read_fails() {
    use std::os::unix::fs::PermissionsExt;

    let home_dir = TempDir::new().unwrap();
    let shnote_dir = home_dir.path().join(".shnote");
    fs::create_dir_all(&shnote_dir).unwrap();

    let config_path = shnote_dir.join("config.toml");
    fs::write(&config_path, "not = \"readable\"\n").unwrap();
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o000)).unwrap();

    shnote_cmd()
        .env("HOME", home_dir.path())
        .args(["--lang", "en", "config", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read config file"));
}

#[test]
fn test_config_get() {
    shnote_cmd()
        .args(["config", "get", "python"])
        .assert()
        .success();
}

#[test]
fn test_config_get_shell() {
    let temp_dir = TempDir::new().unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "get", "shell"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto"));
}

#[test]
fn test_config_get_unknown() {
    shnote_cmd()
        .args(["config", "get", "unknown_key"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown"));
}

#[test]
fn test_config_set_python() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "set", "python", "/usr/bin/python3"])
        .assert()
        .success();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "get", "python"])
        .assert()
        .success()
        .stdout(predicate::str::contains("/usr/bin/python3"));
}

#[test]
fn test_config_set_node() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "set", "node", "/usr/local/bin/node"])
        .assert()
        .success();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "get", "node"])
        .assert()
        .success()
        .stdout(predicate::str::contains("/usr/local/bin/node"));
}

#[test]
fn test_config_set_unknown_key() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "set", "unknown_key", "value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown"));
}

#[test]
fn test_config_set_shell() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "set", "shell", "bash"])
        .assert()
        .success();
}

#[test]
fn test_config_set_invalid_shell() {
    shnote_cmd()
        .args(["config", "set", "shell", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn test_config_set_language() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "set", "language", "zh"])
        .assert()
        .success();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "get", "language"])
        .assert()
        .success()
        .stdout(predicate::str::contains("zh"));
}

#[test]
fn test_config_set_invalid_language() {
    shnote_cmd()
        .args(["config", "set", "language", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn test_config_path() {
    shnote_cmd()
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
fn test_config_reset() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();

    // Set some config first
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "set", "language", "zh"])
        .assert()
        .success();

    // Reset
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "reset"])
        .assert()
        .success();

    // Verify reset to default
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["config", "get", "language"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto"));
}

#[test]
fn test_config_reset_errors_when_home_missing() {
    shnote_cmd()
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .args(["--lang", "en", "config", "reset"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "failed to determine home directory",
        ));
}

// === init command ===
#[test]
fn test_init_claude() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", temp_dir.path())
        .args(["init", "claude"])
        .assert()
        .success();

    let rules_file = temp_dir.path().join(".claude/CLAUDE.md");
    assert!(rules_file.exists());
    let content = fs::read_to_string(rules_file).unwrap();
    assert!(content.contains("shnote rules start"));
}

#[cfg(unix)]
#[test]
fn test_init_claude_writes_rules_when_claude_new_enough() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let tools_dir = TempDir::new().unwrap();

    let claude = tools_dir.path().join("claude");
    fs::write(&claude, "#!/bin/sh\necho \"Claude Code 2.0.64\"\nexit 0\n").unwrap();
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", tools_dir.path())
        .args(["init", "claude"])
        .assert()
        .success();

    let rules_file = temp_dir.path().join(".claude/rules/shnote.md");
    assert!(rules_file.exists());
    let content = fs::read_to_string(rules_file).unwrap();
    assert!(content.contains("shnote"));
    assert!(content.contains("--what"));
}

#[cfg(unix)]
#[test]
fn test_init_claude_writes_claude_md_when_claude_old() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let tools_dir = TempDir::new().unwrap();

    let claude = tools_dir.path().join("claude");
    fs::write(&claude, "#!/bin/sh\necho \"2.0.63\"\nexit 0\n").unwrap();
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", tools_dir.path())
        .args(["init", "claude"])
        .assert()
        .success();

    let rules_file = temp_dir.path().join(".claude/CLAUDE.md");
    assert!(rules_file.exists());
    let content = fs::read_to_string(rules_file).unwrap();
    assert!(content.contains("shnote rules start"));
}

#[test]
fn test_init_codex() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["init", "codex"])
        .assert()
        .success();

    let rules_file = temp_dir.path().join(".codex/AGENTS.md");
    assert!(rules_file.exists());
    let content = fs::read_to_string(rules_file).unwrap();
    assert!(content.contains("shnote rules start"));
}

#[test]
fn test_init_codex_updates_existing() {
    let temp_dir = TempDir::new().unwrap();
    let codex_dir = temp_dir.path().join(".codex");
    fs::create_dir_all(&codex_dir).unwrap();
    let rules_file = codex_dir.join("AGENTS.md");
    fs::write(&rules_file, "Initial content\n").unwrap();

    // First init
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["init", "codex"])
        .assert()
        .success();

    // Second init should update, not duplicate
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--lang", "zh", "init", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("更新"));

    let content = fs::read_to_string(rules_file).unwrap();
    assert_eq!(content.matches("shnote rules start").count(), 1);
}

#[test]
fn test_init_gemini() {
    let temp_dir = TempDir::new().unwrap();
    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["init", "gemini"])
        .assert()
        .success();

    let rules_file = temp_dir.path().join(".gemini/GEMINI.md");
    assert!(rules_file.exists());
}

#[test]
fn test_init_claude_errors_when_create_dir_fails() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join(".claude"), "not a dir").unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", temp_dir.path())
        .args(["--lang", "en", "init", "claude"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to create directory"));
}

#[cfg(unix)]
#[test]
fn test_init_claude_errors_when_write_fails() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".claude/rules")).unwrap();
    fs::create_dir_all(temp_dir.path().join(".claude/rules/shnote.md")).unwrap();

    let tools_dir = TempDir::new().unwrap();
    let claude = tools_dir.path().join("claude");
    fs::write(&claude, "#!/bin/sh\necho \"2.0.64\"\nexit 0\n").unwrap();
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .env("PATH", tools_dir.path())
        .args(["--lang", "en", "init", "claude"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to write file"));
}

#[test]
fn test_init_codex_errors_when_create_dir_fails() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join(".codex"), "not a dir").unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--lang", "en", "init", "codex"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to create directory"));
}

#[test]
fn test_init_codex_errors_when_read_fails() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".codex")).unwrap();
    fs::create_dir_all(temp_dir.path().join(".codex/AGENTS.md")).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--lang", "en", "init", "codex"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read file"));
}

#[test]
fn test_init_gemini_errors_when_create_dir_fails() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join(".gemini"), "not a dir").unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--lang", "en", "init", "gemini"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to create directory"));
}

#[test]
fn test_init_gemini_errors_when_read_fails() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join(".gemini")).unwrap();
    fs::create_dir_all(temp_dir.path().join(".gemini/GEMINI.md")).unwrap();

    shnote_cmd()
        .env("HOME", temp_dir.path())
        .args(["--lang", "en", "init", "gemini"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read file"));
}

#[test]
fn test_init_errors_when_home_missing() {
    shnote_cmd()
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .args(["init", "claude"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "failed to determine home directory",
        ));
}

#[test]
fn test_init_codex_errors_when_home_missing() {
    shnote_cmd()
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .args(["init", "codex"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "failed to determine home directory",
        ));
}

#[test]
fn test_init_gemini_errors_when_home_missing() {
    shnote_cmd()
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .args(["init", "gemini"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "failed to determine home directory",
        ));
}

// === doctor command ===
#[test]
fn test_doctor() {
    shnote_cmd()
        .arg("doctor")
        .assert()
        .code(predicate::in_iter([0, 1])); // May succeed or fail depending on environment
}

#[cfg(unix)]
#[test]
fn test_doctor_success() {
    use std::os::unix::fs::PermissionsExt;

    let home_dir = TempDir::new().unwrap();
    let tools_dir = TempDir::new().unwrap();

    let python3 = tools_dir.path().join("python3");
    fs::write(&python3, "#!/bin/sh\necho \"Python 3.0\" >&2\nexit 0\n").unwrap();
    fs::set_permissions(&python3, fs::Permissions::from_mode(0o755)).unwrap();

    let node = tools_dir.path().join("node");
    fs::write(&node, "#!/bin/sh\necho \"v1.0\"\nexit 0\n").unwrap();
    fs::set_permissions(&node, fs::Permissions::from_mode(0o755)).unwrap();

    let bash = tools_dir.path().join("bash");
    fs::write(&bash, "#!/bin/sh\necho \"bash 1.0\"\nexit 0\n").unwrap();
    fs::set_permissions(&bash, fs::Permissions::from_mode(0o755)).unwrap();

    // Provide pueue binaries in shnote's bin directory.
    let bin_dir = home_dir.path().join(".shnote/bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let pueue = bin_dir.join("pueue");
    fs::write(&pueue, "#!/bin/sh\necho \"pueue 4.0\"\nexit 0\n").unwrap();
    fs::set_permissions(&pueue, fs::Permissions::from_mode(0o755)).unwrap();
    let pueued = bin_dir.join("pueued");
    fs::write(&pueued, "#!/bin/sh\necho \"pueued 4.0\"\nexit 0\n").unwrap();
    fs::set_permissions(&pueued, fs::Permissions::from_mode(0o755)).unwrap();

    shnote_cmd()
        .env("HOME", home_dir.path())
        .env("PATH", tools_dir.path())
        .env("SHELL", &bash)
        .args(["--lang", "en", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("All dependencies OK!"));
}

#[cfg(unix)]
#[test]
fn test_doctor_failure_exit_code() {
    let home_dir = TempDir::new().unwrap();
    let empty_path = TempDir::new().unwrap();

    shnote_cmd()
        .env("HOME", home_dir.path())
        .env("PATH", empty_path.path())
        .env_remove("SHELL")
        .args(["--lang", "en", "doctor"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Some dependencies have issues"));
}

#[cfg(unix)]
#[test]
fn test_setup_creates_pueue_binaries() {
    use std::os::unix::fs::PermissionsExt;

    let home_dir = TempDir::new().unwrap();
    let tools_dir = TempDir::new().unwrap();

    // Provide fake curl + shasum in case setup falls back to download path.
    let curl = tools_dir.path().join("curl");
    fs::write(
        &curl,
        "#!/bin/sh\n\
dest=\"\"\n\
while [ \"$#\" -gt 0 ]; do\n\
  if [ \"$1\" = \"-o\" ]; then\n\
    dest=\"$2\"\n\
    break\n\
  fi\n\
  shift\n\
done\n\
if [ -z \"$dest\" ]; then\n\
  exit 2\n\
fi\n\
echo \"dummy\" > \"$dest\"\n\
exit 0\n",
    )
    .unwrap();
    fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();

    let shasum = tools_dir.path().join("shasum");
    // Platform-specific checksums from src/pueue_embed.rs (v4.0.1)
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let (pueue_sha, pueued_sha) = (
        "4306f593b6a6b6db9d641889e33fe3a2effa6423888b8f82391fa57951ef1a9b",
        "dc14a7873a4a474ae42e7a6ee5778c2af2d53049182ecaa2d061f4803f04bf23",
    );
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let (pueue_sha, pueued_sha) = (
        "25f07f7e93f916d6189acc11846aab6ebee975b0cc5867cf40a96b5c70f3b55c",
        "3e50d3bfadd1e417c8561aed2c1f4371605e8002f7fd793f39045719af5436a8",
    );
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let (pueue_sha, pueued_sha) = (
        "16aea6654b3915c6495bb2f456184fd7f3d418de3f74afb5eab04ae953cdfedf",
        "8a97b176f55929e37cda49577b28b66ea345151adf766b9d8efa8c9d81525a0b",
    );
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    let (pueue_sha, pueued_sha) = (
        "666af79b5a0246efa61a8589e51a190e3174bf80ad1c78b264204e7d312d43a9",
        "8d3811f2ad57ef72ed171f446f19676ef755e189286d1c31a1e478ed57465bdb",
    );
    fs::write(
        &shasum,
        format!(
            "#!/bin/sh\n\
file=\"$3\"\n\
case \"$file\" in\n\
  *pueued) echo \"{pueued_sha}  $file\" ;;\n\
  *pueue) echo \"{pueue_sha}  $file\" ;;\n\
  *) echo \"{pueue_sha}  $file\" ;;\n\
esac\n\
exit 0\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&shasum, fs::Permissions::from_mode(0o755)).unwrap();

    shnote_cmd()
        .env("HOME", home_dir.path())
        .env("PATH", tools_dir.path())
        .args(["--lang", "en", "setup"])
        .assert()
        .success();

    let bin_dir = home_dir.path().join(".shnote/bin");
    assert!(bin_dir.join("pueue").exists());
    assert!(bin_dir.join("pueued").exists());
}

#[cfg(unix)]
#[test]
fn test_setup_errors_when_shnote_home_is_file() {
    let home_dir = TempDir::new().unwrap();
    fs::write(home_dir.path().join(".shnote"), "not a dir").unwrap();

    shnote_cmd()
        .env("HOME", home_dir.path())
        .args(["--lang", "en", "setup"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed"));
}

// === completions command ===
#[test]
fn test_completions_bash() {
    shnote_cmd()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shnote"));
}

#[test]
fn test_completions_zsh() {
    shnote_cmd()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shnote"));
}

#[test]
fn test_completions_fish() {
    shnote_cmd()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shnote"));
}

#[test]
fn test_completions_powershell() {
    shnote_cmd()
        .args(["completions", "powershell"])
        .assert()
        .success();
}

#[test]
fn test_completions_elvish() {
    shnote_cmd()
        .args(["completions", "elvish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shnote"));
}

// === Error cases ===
#[test]
fn test_what_why_on_non_exec_command() {
    shnote_cmd()
        .args(["--what", "test", "--why", "test", "config", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--what"));
}

#[test]
fn test_what_why_on_non_exec_command_english() {
    shnote_cmd()
        .args([
            "--lang", "en", "--what", "test", "--why", "test", "config", "list",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("only accepted"));
}

#[test]
fn test_py_requires_source_english() {
    shnote_cmd()
        .args(["--lang", "en", "--what", "test", "--why", "test", "py"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exactly one of"));
}

#[test]
fn test_run_nonexistent_command() {
    shnote_cmd()
        .args([
            "--what",
            "test",
            "--why",
            "test",
            "run",
            "nonexistent_command_xyz",
        ])
        .assert()
        .failure();
}

#[test]
fn test_run_nonexistent_command_english() {
    shnote_cmd()
        .args([
            "--lang",
            "en",
            "--what",
            "test",
            "--why",
            "test",
            "run",
            "nonexistent_command_xyz",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to execute"));
}
