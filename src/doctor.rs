use std::path::PathBuf;
use std::process::Command;

use which::which;

use crate::config::Config;
use crate::i18n::I18n;
use crate::pueue::{find_pueue, find_pueued};
use crate::shell::{detect_shell, get_shell_version};

pub struct CheckResult {
    pub name: String,
    pub ok: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub error: Option<String>,
}

impl CheckResult {
    fn success(name: &str, path: PathBuf, version: Option<String>) -> Self {
        Self {
            name: name.to_string(),
            ok: true,
            path: Some(path),
            version,
            error: None,
        }
    }

    fn failure(name: &str, error: &str) -> Self {
        Self {
            name: name.to_string(),
            ok: false,
            path: None,
            version: None,
            error: Some(error.to_string()),
        }
    }
}

pub fn run_doctor(i18n: &I18n, config: &Config) -> Vec<CheckResult> {
    vec![
        check_python(i18n, config),
        check_node(i18n, config),
        check_shell(i18n, config),
        check_pueue(i18n),
        check_pueued(i18n),
    ]
}

pub fn print_doctor_results(i18n: &I18n, results: &[CheckResult]) {
    let mut all_ok = true;

    for result in results {
        if result.ok {
            let path_str = result
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let version_str = result
                .version
                .as_ref()
                .map(|v| format!(" ({})", v))
                .unwrap_or_default();
            println!("✓ {}: {}{}", result.name, path_str, version_str);
        } else {
            all_ok = false;
            let error_str = result.error.as_deref().unwrap_or("unknown error");
            println!("✗ {}: {}", result.name, error_str);
        }
    }

    println!();
    if all_ok {
        println!("{}", i18n.doctor_all_ok());
    } else {
        println!("{}", i18n.doctor_has_issues());
    }
}

fn check_python(i18n: &I18n, config: &Config) -> CheckResult {
    let python_cmd = &config.paths.python;

    // Try configured path first
    let path = if PathBuf::from(python_cmd).is_absolute() {
        let p = PathBuf::from(python_cmd);
        if p.exists() {
            Some(p)
        } else {
            None
        }
    } else {
        which(python_cmd).ok()
    };

    // Try fallbacks
    let path = path
        .or_else(|| which("python3").ok())
        .or_else(|| which("python").ok());

    match path {
        Some(p) => {
            let version = get_interpreter_version(&p, "--version");
            CheckResult::success("python", p, version)
        }
        None => CheckResult::failure("python", i18n.doctor_not_found_in_path()),
    }
}

fn check_node(i18n: &I18n, config: &Config) -> CheckResult {
    let node_cmd = &config.paths.node;

    let path = if PathBuf::from(node_cmd).is_absolute() {
        let p = PathBuf::from(node_cmd);
        if p.exists() {
            Some(p)
        } else {
            None
        }
    } else {
        which(node_cmd).ok()
    };

    let path = path.or_else(|| which("node").ok());

    match path {
        Some(p) => {
            let version = get_interpreter_version(&p, "--version");
            CheckResult::success("node", p, version)
        }
        None => CheckResult::failure("node", i18n.doctor_not_found_in_path()),
    }
}

fn check_shell(i18n: &I18n, config: &Config) -> CheckResult {
    match detect_shell(i18n, &config.paths.shell) {
        Ok((shell_type, path)) => {
            let version = get_shell_version(&shell_type, &path);
            CheckResult::success("shell", path, version)
        }
        Err(e) => CheckResult::failure("shell", &e.to_string()),
    }
}

fn check_pueue(i18n: &I18n) -> CheckResult {
    match find_pueue() {
        Some(path) => {
            let version = get_interpreter_version(&path, "--version");
            CheckResult::success("pueue", path, version)
        }
        None => CheckResult::failure("pueue", i18n.doctor_pueue_not_found()),
    }
}

fn check_pueued(i18n: &I18n) -> CheckResult {
    match find_pueued() {
        Some(path) => {
            let version = get_interpreter_version(&path, "--version");
            CheckResult::success("pueued", path, version)
        }
        None => CheckResult::failure("pueued", i18n.doctor_pueue_not_found()),
    }
}

fn get_interpreter_version(path: &PathBuf, flag: &str) -> Option<String> {
    let output = Command::new(path).arg(flag).output().ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Some tools output version to stderr
        let version_str = if stdout.trim().is_empty() {
            stderr.trim()
        } else {
            stdout.trim()
        };
        // Return first line only
        version_str.lines().next().map(|s| s.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{pueue_binary_name, pueued_binary_name, shnote_bin_dir};
    use crate::i18n::Lang;
    use crate::test_support::{env_lock, EnvVarGuard};
    use tempfile::TempDir;

    #[cfg(unix)]
    use crate::test_support::write_executable;

    fn test_i18n() -> I18n {
        I18n::new(Lang::En)
    }

    #[test]
    fn check_result_success() {
        let result = CheckResult::success(
            "test",
            PathBuf::from("/usr/bin/test"),
            Some("1.0.0".to_string()),
        );
        assert!(result.ok);
        assert_eq!(result.name, "test");
        assert_eq!(result.path, Some(PathBuf::from("/usr/bin/test")));
        assert_eq!(result.version, Some("1.0.0".to_string()));
        assert!(result.error.is_none());
    }

    #[test]
    fn check_result_failure() {
        let result = CheckResult::failure("test", "not found");
        assert!(!result.ok);
        assert_eq!(result.name, "test");
        assert!(result.path.is_none());
        assert!(result.version.is_none());
        assert_eq!(result.error, Some("not found".to_string()));
    }

    #[test]
    fn run_doctor_returns_results() {
        let i18n = test_i18n();
        let config = Config::default();
        let results = run_doctor(&i18n, &config);

        // Should always return 5 results (python, node, shell, pueue, pueued)
        assert_eq!(results.len(), 5);

        // Check names
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"python"));
        assert!(names.contains(&"node"));
        assert!(names.contains(&"shell"));
        assert!(names.contains(&"pueue"));
        assert!(names.contains(&"pueued"));
    }

    #[test]
    fn print_doctor_results_with_failures() {
        let i18n = test_i18n();
        let results = vec![
            CheckResult::success(
                "test1",
                PathBuf::from("/usr/bin/test"),
                Some("1.0".to_string()),
            ),
            CheckResult::failure("test2", "not found"),
        ];

        // This will print to stdout, we just test it doesn't panic
        print_doctor_results(&i18n, &results);
    }

    #[test]
    fn print_doctor_results_all_success() {
        let i18n = test_i18n();
        let results = vec![CheckResult::success(
            "test1",
            PathBuf::from("/usr/bin/test"),
            Some("1.0".to_string()),
        )];

        print_doctor_results(&i18n, &results);
    }

    #[test]
    fn get_interpreter_version_with_invalid_path() {
        let result = get_interpreter_version(&PathBuf::from("/nonexistent"), "--version");
        assert!(result.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn get_interpreter_version_returns_none_on_nonzero_exit() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let tool = temp_dir.path().join("tool");
        write_executable(&tool, "#!/bin/sh\nexit 1\n").unwrap();

        assert!(get_interpreter_version(&tool, "--version").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn get_interpreter_version_prefers_stderr_when_stdout_empty() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let tool = temp_dir.path().join("tool");
        write_executable(&tool, "#!/bin/sh\necho \"v1.2.3\" 1>&2\nexit 0\n").unwrap();

        assert_eq!(
            get_interpreter_version(&tool, "--version"),
            Some("v1.2.3".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn get_interpreter_version_returns_none_when_no_output() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let tool = temp_dir.path().join("tool");
        write_executable(&tool, "#!/bin/sh\nexit 0\n").unwrap();

        assert!(get_interpreter_version(&tool, "--version").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn check_python_reports_failure_when_not_found() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());

        let mut config = Config::default();
        config.paths.python = "python-does-not-exist".to_string();

        let result = check_python(&i18n, &config);
        assert!(!result.ok);
        assert_eq!(result.name, "python");
        assert_eq!(
            result.error.as_deref(),
            Some(i18n.doctor_not_found_in_path())
        );
    }

    #[cfg(unix)]
    #[test]
    fn check_python_uses_configured_absolute_path() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let python = temp_dir.path().join("python3");
        write_executable(&python, "#!/bin/sh\necho \"Python 3.99.0\" 1>&2\nexit 0\n").unwrap();

        let mut config = Config::default();
        config.paths.python = python.display().to_string();

        let result = check_python(&i18n, &config);
        assert!(result.ok);
        assert_eq!(result.path, Some(python));
        assert_eq!(result.version, Some("Python 3.99.0".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn check_python_absolute_nonexistent_path_reports_failure() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());

        let mut config = Config::default();
        config.paths.python = "/nonexistent/python".to_string();

        let result = check_python(&i18n, &config);
        assert!(!result.ok);
        assert_eq!(result.name, "python");
        assert_eq!(
            result.error.as_deref(),
            Some(i18n.doctor_not_found_in_path())
        );
    }

    #[cfg(unix)]
    #[test]
    fn check_node_reports_failure_when_not_found() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());

        let mut config = Config::default();
        config.paths.node = "node-does-not-exist".to_string();

        let result = check_node(&i18n, &config);
        assert!(!result.ok);
        assert_eq!(result.name, "node");
        assert_eq!(
            result.error.as_deref(),
            Some(i18n.doctor_not_found_in_path())
        );
    }

    #[cfg(unix)]
    #[test]
    fn check_node_uses_configured_absolute_path() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let node = temp_dir.path().join("node");
        write_executable(&node, "#!/bin/sh\necho \"v20.0.0\"\nexit 0\n").unwrap();

        let mut config = Config::default();
        config.paths.node = node.display().to_string();

        let result = check_node(&i18n, &config);
        assert!(result.ok);
        assert_eq!(result.path, Some(node));
        assert_eq!(result.version, Some("v20.0.0".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn check_node_absolute_nonexistent_path_reports_failure() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());

        let mut config = Config::default();
        config.paths.node = "/nonexistent/node".to_string();

        let result = check_node(&i18n, &config);
        assert!(!result.ok);
        assert_eq!(result.name, "node");
        assert_eq!(
            result.error.as_deref(),
            Some(i18n.doctor_not_found_in_path())
        );
    }

    #[cfg(unix)]
    #[test]
    fn check_shell_reports_failure_when_explicit_shell_missing() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());

        let mut config = Config::default();
        config.paths.shell = "bash".to_string();

        let result = check_shell(&i18n, &config);
        assert!(!result.ok);
        assert_eq!(result.name, "shell");
        assert_eq!(result.error, Some(i18n.err_shell_not_in_path("bash")));
    }

    #[cfg(unix)]
    #[test]
    fn check_pueue_prefers_shnote_bin_dir() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let bin_dir = shnote_bin_dir().unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();
        let pueue_path = bin_dir.join(pueue_binary_name());
        write_executable(&pueue_path, "#!/bin/sh\necho \"pueue 4.0.1\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());

        let result = check_pueue(&i18n);
        assert!(result.ok);
        assert_eq!(result.path, Some(pueue_path));
        assert_eq!(result.version, Some("pueue 4.0.1".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn check_pueue_falls_back_to_path() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let home_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());

        let path_dir = TempDir::new().unwrap();
        let pueue = path_dir.path().join("pueue");
        write_executable(&pueue, "#!/bin/sh\necho \"pueue 4.0.1\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());

        let result = check_pueue(&i18n);
        assert!(result.ok);
        assert_eq!(result.path, Some(pueue));
    }

    #[cfg(unix)]
    #[test]
    fn check_pueue_falls_back_to_path_when_bin_dir_exists_without_binary() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let home_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());
        std::fs::create_dir_all(shnote_bin_dir().unwrap()).unwrap();

        let path_dir = TempDir::new().unwrap();
        let pueue = path_dir.path().join("pueue");
        write_executable(&pueue, "#!/bin/sh\necho \"pueue 4.0.1\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());

        let result = check_pueue(&i18n);
        assert!(result.ok);
        assert_eq!(result.path, Some(pueue));
    }

    #[cfg(unix)]
    #[test]
    fn check_pueue_reports_failure_when_missing() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let home_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());

        let path_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());

        let result = check_pueue(&i18n);
        assert!(!result.ok);
        assert_eq!(result.name, "pueue");
        assert_eq!(result.error.as_deref(), Some(i18n.doctor_pueue_not_found()));
    }

    #[cfg(unix)]
    #[test]
    fn check_pueued_prefers_shnote_bin_dir() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let bin_dir = shnote_bin_dir().unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();
        let pueued_path = bin_dir.join(pueued_binary_name());
        write_executable(&pueued_path, "#!/bin/sh\necho \"pueued 4.0.1\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());

        let result = check_pueued(&i18n);
        assert!(result.ok);
        assert_eq!(result.path, Some(pueued_path));
        assert_eq!(result.version, Some("pueued 4.0.1".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn check_pueued_falls_back_to_path() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let home_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());

        let path_dir = TempDir::new().unwrap();
        let pueued = path_dir.path().join("pueued");
        write_executable(&pueued, "#!/bin/sh\necho \"pueued 4.0.1\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());

        let result = check_pueued(&i18n);
        assert!(result.ok);
        assert_eq!(result.path, Some(pueued));
    }

    #[cfg(unix)]
    #[test]
    fn check_pueued_falls_back_to_path_when_bin_dir_exists_without_binary() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let home_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());
        std::fs::create_dir_all(shnote_bin_dir().unwrap()).unwrap();

        let path_dir = TempDir::new().unwrap();
        let pueued = path_dir.path().join("pueued");
        write_executable(&pueued, "#!/bin/sh\necho \"pueued 4.0.1\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());

        let result = check_pueued(&i18n);
        assert!(result.ok);
        assert_eq!(result.path, Some(pueued));
    }

    #[cfg(unix)]
    #[test]
    fn check_pueued_reports_failure_when_missing() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let home_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());

        let path_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());

        let result = check_pueued(&i18n);
        assert!(!result.ok);
        assert_eq!(result.name, "pueued");
        assert_eq!(result.error.as_deref(), Some(i18n.doctor_pueue_not_found()));
    }
}
