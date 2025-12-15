use std::env;
use std::path::PathBuf;

use anyhow::Result;
use which::which;

use crate::i18n::I18n;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellType {
    Sh,
    Bash,
    Zsh,
    Pwsh,
    Cmd,
}

impl ShellType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sh" => Some(Self::Sh),
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "pwsh" | "powershell" => Some(Self::Pwsh),
            "cmd" | "cmd.exe" => Some(Self::Cmd),
            _ => None,
        }
    }

    pub fn command_name(&self) -> &'static str {
        match self {
            Self::Sh => "sh",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Pwsh => "pwsh",
            Self::Cmd => "cmd",
        }
    }

    /// Returns the argument to pass inline script code
    #[allow(dead_code)]
    pub fn code_flag(&self) -> &'static str {
        match self {
            Self::Sh | Self::Bash | Self::Zsh => "-c",
            Self::Pwsh => "-Command",
            Self::Cmd => "/C",
        }
    }
}

/// Detect shell from configuration or environment
pub fn detect_shell(i18n: &I18n, config_shell: &str) -> Result<(ShellType, PathBuf)> {
    if config_shell != "auto" {
        if let Some(shell_type) = ShellType::from_str(config_shell) {
            let path = resolve_shell_path(i18n, &shell_type)?;
            return Ok((shell_type, path));
        }
    }

    // Auto-detect
    auto_detect_shell(i18n)
}

fn auto_detect_shell(i18n: &I18n) -> Result<(ShellType, PathBuf)> {
    #[cfg(unix)]
    {
        // On Unix, try $SHELL first, then fall back to common shells.
        let from_env = env::var("SHELL").ok().and_then(|shell_path| {
            let path = PathBuf::from(shell_path);
            if !path.exists() {
                return None;
            }

            let shell_type = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(ShellType::from_str);

            shell_type.map(|shell_type| (shell_type, path))
        });

        if let Some(detected) = from_env {
            Ok(detected)
        } else {
            let candidates = [ShellType::Zsh, ShellType::Bash, ShellType::Sh];
            let detected = candidates.into_iter().find_map(|shell_type| {
                which(shell_type.command_name())
                    .ok()
                    .map(|path| (shell_type, path))
            });

            match detected {
                Some(detected) => Ok(detected),
                None => anyhow::bail!("{}", i18n.err_no_shell_unix()),
            }
        }
    }

    #[cfg(windows)]
    {
        // On Windows, try pwsh -> powershell -> cmd
        let candidates = [
            (ShellType::Pwsh, "pwsh"),
            (ShellType::Pwsh, "powershell"),
            (ShellType::Cmd, "cmd"),
        ];

        for (shell_type, cmd) in candidates {
            if let Ok(path) = which(cmd) {
                return Ok((shell_type, path));
            }
        }

        // cmd.exe should always exist on Windows
        let cmd_path = PathBuf::from(r"C:\Windows\System32\cmd.exe");
        if cmd_path.exists() {
            return Ok((ShellType::Cmd, cmd_path));
        }

        anyhow::bail!("{}", i18n.err_no_shell_windows())
    }
}

fn resolve_shell_path(i18n: &I18n, shell_type: &ShellType) -> Result<PathBuf> {
    let cmd = shell_type.command_name();
    which(cmd).map_err(|_| anyhow::anyhow!("{}", i18n.err_shell_not_in_path(cmd)))
}

/// Get version string from shell
pub fn get_shell_version(shell_type: &ShellType, path: &PathBuf) -> Option<String> {
    use std::process::Command;

    let output = match shell_type {
        ShellType::Sh | ShellType::Bash | ShellType::Zsh => {
            Command::new(path).arg("--version").output().ok()?
        }
        ShellType::Pwsh => Command::new(path).arg("--version").output().ok()?,
        ShellType::Cmd => {
            // cmd doesn't have a simple version flag
            return Some("Windows CMD".to_string());
        }
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Return first line
        stdout.lines().next().map(|s| s.trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{env_lock, EnvVarGuard};
    use tempfile::TempDir;

    #[cfg(unix)]
    use crate::test_support::write_executable;

    #[test]
    fn shell_type_from_str() {
        assert_eq!(ShellType::from_str("bash"), Some(ShellType::Bash));
        assert_eq!(ShellType::from_str("BASH"), Some(ShellType::Bash));
        assert_eq!(ShellType::from_str("sh"), Some(ShellType::Sh));
        assert_eq!(ShellType::from_str("zsh"), Some(ShellType::Zsh));
        assert_eq!(ShellType::from_str("pwsh"), Some(ShellType::Pwsh));
        assert_eq!(ShellType::from_str("cmd"), Some(ShellType::Cmd));
        assert_eq!(ShellType::from_str("cmd.exe"), Some(ShellType::Cmd));
        assert_eq!(ShellType::from_str("invalid"), None);
    }

    #[test]
    fn shell_type_code_flag() {
        assert_eq!(ShellType::Bash.code_flag(), "-c");
        assert_eq!(ShellType::Pwsh.code_flag(), "-Command");
        assert_eq!(ShellType::Cmd.code_flag(), "/C");
    }

    #[test]
    fn shell_type_command_name() {
        assert_eq!(ShellType::Bash.command_name(), "bash");
        assert_eq!(ShellType::Zsh.command_name(), "zsh");
        assert_eq!(ShellType::Sh.command_name(), "sh");
        assert_eq!(ShellType::Pwsh.command_name(), "pwsh");
        assert_eq!(ShellType::Cmd.command_name(), "cmd");
    }

    #[cfg(unix)]
    #[test]
    fn detect_shell_with_invalid_config_falls_back_to_auto() {
        use crate::i18n::Lang;

        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let zsh = temp_dir.path().join("zsh");
        write_executable(&zsh, "#!/bin/sh\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());
        let _shell_guard = EnvVarGuard::set("SHELL", zsh.as_os_str());

        let (shell_type, resolved) = detect_shell(&i18n, "invalid").unwrap();
        assert_eq!(shell_type, ShellType::Zsh);
        assert_eq!(resolved, zsh);
    }

    #[cfg(unix)]
    #[test]
    fn detect_shell_explicit_bash_uses_path() {
        use crate::i18n::Lang;

        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let bash = temp_dir.path().join("bash");
        write_executable(&bash, "#!/bin/sh\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());
        let _shell_guard = EnvVarGuard::remove("SHELL");

        let (shell_type, resolved) = detect_shell(&i18n, "bash").unwrap();
        assert_eq!(shell_type, ShellType::Bash);
        assert_eq!(resolved, bash);
    }

    #[cfg(unix)]
    #[test]
    fn auto_detect_shell_ignores_nonexistent_shell_env_and_falls_back_to_path() {
        use crate::i18n::Lang;

        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);
        let temp_dir = TempDir::new().unwrap();

        let bash = temp_dir.path().join("bash");
        write_executable(&bash, "#!/bin/sh\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());
        let _shell_guard = EnvVarGuard::set("SHELL", "/nonexistent/bash");

        let (shell_type, resolved) = detect_shell(&i18n, "auto").unwrap();
        assert_eq!(shell_type, ShellType::Bash);
        assert_eq!(resolved, bash);
    }

    #[cfg(unix)]
    #[test]
    fn auto_detect_shell_errors_when_no_shell_available() {
        use crate::i18n::Lang;

        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);
        let temp_dir = TempDir::new().unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());
        let _shell_guard = EnvVarGuard::set("SHELL", "/nonexistent/bash");
        assert!(detect_shell(&i18n, "auto").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn auto_detect_shell_works_when_shell_env_unset() {
        use crate::i18n::Lang;

        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);
        let temp_dir = TempDir::new().unwrap();

        let sh = temp_dir.path().join("sh");
        write_executable(&sh, "#!/bin/sh\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());
        let _shell_guard = EnvVarGuard::remove("SHELL");

        let (shell_type, resolved) = detect_shell(&i18n, "auto").unwrap();
        assert_eq!(shell_type, ShellType::Sh);
        assert_eq!(resolved, sh);
    }

    #[cfg(unix)]
    #[test]
    fn auto_detect_shell_uses_shell_env_when_valid() {
        use crate::i18n::Lang;

        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let bash = temp_dir.path().join("bash");
        write_executable(&bash, "#!/bin/sh\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());
        let _shell_guard = EnvVarGuard::set("SHELL", bash.as_os_str());

        let (shell_type, resolved) = detect_shell(&i18n, "auto").unwrap();
        assert_eq!(shell_type, ShellType::Bash);
        assert_eq!(resolved, bash);
    }

    #[cfg(unix)]
    #[test]
    fn get_shell_version_bash_returns_first_line() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();

        let bash = temp_dir.path().join("bash");
        write_executable(
            &bash,
            "#!/bin/sh\necho \"GNU bash, version 5.2.0\"\necho \"second line\"\nexit 0\n",
        )
        .unwrap();

        let version = get_shell_version(&ShellType::Bash, &bash);
        assert_eq!(version, Some("GNU bash, version 5.2.0".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn get_shell_version_pwsh() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();

        let pwsh = temp_dir.path().join("pwsh");
        write_executable(&pwsh, "#!/bin/sh\necho \"PowerShell 7.4.0\"\nexit 0\n").unwrap();

        let version = get_shell_version(&ShellType::Pwsh, &pwsh);
        assert_eq!(version, Some("PowerShell 7.4.0".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn get_shell_version_returns_none_on_failure() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();

        let sh = temp_dir.path().join("sh");
        write_executable(&sh, "#!/bin/sh\nexit 1\n").unwrap();

        let version = get_shell_version(&ShellType::Sh, &sh);
        assert!(version.is_none());
    }

    #[test]
    fn get_shell_version_returns_none_when_command_cannot_spawn() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nope");

        assert!(get_shell_version(&ShellType::Bash, &nonexistent).is_none());
        assert!(get_shell_version(&ShellType::Pwsh, &nonexistent).is_none());
    }

    #[test]
    fn get_shell_version_cmd() {
        let version = get_shell_version(&ShellType::Cmd, &PathBuf::from("dummy"));
        assert_eq!(version, Some("Windows CMD".to_string()));
    }
}
