use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::i18n::I18n;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub paths: PathsConfig,

    #[serde(default)]
    pub i18n: I18nConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathsConfig {
    /// Python interpreter path or command name
    #[serde(default = "PathsConfig::default_python")]
    pub python: String,

    /// Node.js interpreter path or command name
    #[serde(default = "PathsConfig::default_node")]
    pub node: String,

    /// Shell type: auto | sh | bash | zsh | pwsh | cmd
    #[serde(default = "PathsConfig::default_shell")]
    pub shell: String,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            python: Self::default_python(),
            node: Self::default_node(),
            shell: Self::default_shell(),
        }
    }
}

impl PathsConfig {
    fn default_python() -> String {
        "python3".to_string()
    }

    fn default_node() -> String {
        "node".to_string()
    }

    fn default_shell() -> String {
        "auto".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct I18nConfig {
    /// Language: zh | en | auto
    #[serde(default = "I18nConfig::default_language")]
    pub language: String,
}

impl Default for I18nConfig {
    fn default() -> Self {
        Self {
            language: Self::default_language(),
        }
    }
}

impl I18nConfig {
    fn default_language() -> String {
        "auto".to_string()
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Config::default());
        }
        let contents = fs::read_to_string(&path)
            .context(format!("failed to read config file: {}", path.display()))?;
        toml::from_str(&contents).context(format!("failed to parse config file: {}", path.display()))
    }

    pub fn save(&self, i18n: &I18n) -> Result<()> {
        let parent = shnote_home()?;
        let path = parent.join("config.toml");
        fs::create_dir_all(&parent).context(i18n.err_create_config_dir(
            &parent.display().to_string(),
        ))?;
        #[allow(clippy::expect_used)]
        let msg = i18n.err_serialize_config();
        let contents = toml::to_string_pretty(self).expect(msg);
        fs::write(&path, contents).context(i18n.err_write_config(&path.display().to_string()))
    }

    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "python" => Some(self.paths.python.clone()),
            "node" => Some(self.paths.node.clone()),
            "shell" => Some(self.paths.shell.clone()),
            "language" => Some(self.i18n.language.clone()),
            _ => None,
        }
    }

    pub fn set(&mut self, i18n: &I18n, key: &str, value: &str) -> Result<bool> {
        match key {
            "python" => {
                self.paths.python = value.to_string();
                Ok(true)
            }
            "node" => {
                self.paths.node = value.to_string();
                Ok(true)
            }
            "shell" => {
                let valid = ["auto", "sh", "bash", "zsh", "pwsh", "cmd"];
                if !valid.contains(&value) {
                    anyhow::bail!("{}", i18n.err_invalid_shell_value(value, &valid.join(", ")));
                }
                self.paths.shell = value.to_string();
                Ok(true)
            }
            "language" => {
                let valid = ["auto", "zh", "en"];
                if !valid.contains(&value) {
                    anyhow::bail!(
                        "{}",
                        i18n.err_invalid_language_value(value, &valid.join(", "))
                    );
                }
                self.i18n.language = value.to_string();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn list(&self) -> Vec<(String, String)> {
        vec![
            ("python".to_string(), self.paths.python.clone()),
            ("node".to_string(), self.paths.node.clone()),
            ("shell".to_string(), self.paths.shell.clone()),
            ("language".to_string(), self.i18n.language.clone()),
        ]
    }

    pub fn reset(i18n: &I18n) -> Result<Self> {
        let config = Config::default();
        config.save(i18n)?;
        Ok(config)
    }
}

pub fn config_path() -> Result<PathBuf> {
    Ok(shnote_home()?.join("config.toml"))
}

pub fn shnote_home() -> Result<PathBuf> {
    let home = home_dir()?;
    Ok(home.join(".shnote"))
}

pub fn home_dir() -> Result<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .context("failed to determine home directory")?;
    Ok(PathBuf::from(home))
}

pub fn shnote_bin_dir() -> Result<PathBuf> {
    Ok(shnote_home()?.join("bin"))
}

pub fn pueue_binary_name() -> &'static str {
    #[cfg(windows)]
    {
        "pueue.exe"
    }
    #[cfg(not(windows))]
    {
        "pueue"
    }
}

pub fn pueued_binary_name() -> &'static str {
    #[cfg(windows)]
    {
        "pueued.exe"
    }
    #[cfg(not(windows))]
    {
        "pueued"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::test_support::{env_lock, EnvVarGuard};
    use std::fs;

    fn test_i18n() -> I18n {
        I18n::new(Lang::En)
    }

    #[test]
    fn config_default_values() {
        let config = Config::default();
        assert_eq!(config.paths.python, "python3");
        assert_eq!(config.paths.node, "node");
        assert_eq!(config.paths.shell, "auto");
        assert_eq!(config.i18n.language, "auto");
    }

    #[test]
    fn config_get_set() {
        let i18n = test_i18n();
        let mut config = Config::default();

        assert_eq!(config.get("python"), Some("python3".to_string()));
        assert_eq!(config.get("shell"), Some("auto".to_string()));
        assert_eq!(config.get("language"), Some("auto".to_string()));

        config.set(&i18n, "python", "/usr/bin/python3").unwrap();
        assert_eq!(config.get("python"), Some("/usr/bin/python3".to_string()));

        config.set(&i18n, "node", "/usr/bin/node").unwrap();
        assert_eq!(config.get("node"), Some("/usr/bin/node".to_string()));

        assert!(config.get("nonexistent").is_none());
        assert!(!config.set(&i18n, "nonexistent", "value").unwrap());
    }

    #[test]
    fn config_set_validates_shell() {
        let i18n = test_i18n();
        let mut config = Config::default();

        assert!(config.set(&i18n, "shell", "bash").is_ok());
        assert!(config.set(&i18n, "shell", "invalid").is_err());
    }

    #[test]
    fn config_set_validates_language() {
        let i18n = test_i18n();
        let mut config = Config::default();

        assert!(config.set(&i18n, "language", "zh").is_ok());
        assert!(config.set(&i18n, "language", "invalid").is_err());
    }

    #[test]
    fn config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.paths.python, config.paths.python);
    }

    #[test]
    fn config_list() {
        let config = Config::default();
        let list = config.list();
        assert_eq!(list.len(), 4);
        assert!(list.contains(&("python".to_string(), "python3".to_string())));
        assert!(list.contains(&("node".to_string(), "node".to_string())));
    }

    #[test]
    fn config_reset() {
        use tempfile::TempDir;
        let i18n = test_i18n();
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        Config::reset(&i18n).unwrap();

        let config = Config::load().unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn config_path_is_under_shnote_home() {
        use tempfile::TempDir;
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let path = config_path().unwrap();
        assert_eq!(path, temp_dir.path().join(".shnote/config.toml"));
    }

    #[test]
    fn shnote_bin_dir_is_under_shnote_home() {
        use tempfile::TempDir;
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let bin_dir = shnote_bin_dir().unwrap();
        assert_eq!(bin_dir, temp_dir.path().join(".shnote/bin"));
    }

    #[test]
    fn pueue_binary_names_are_platform_specific() {
        #[cfg(windows)]
        {
            assert_eq!(pueue_binary_name(), "pueue.exe");
            assert_eq!(pueued_binary_name(), "pueued.exe");
        }

        #[cfg(not(windows))]
        {
            assert_eq!(pueue_binary_name(), "pueue");
            assert_eq!(pueued_binary_name(), "pueued");
        }
    }

    #[test]
    fn config_load_returns_default_when_missing() {
        use tempfile::TempDir;
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let config = Config::load().unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn config_load_fails_when_config_path_is_directory() {
        use tempfile::TempDir;
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let path = config_path().unwrap();
        fs::create_dir_all(&path).unwrap();

        let err = Config::load().unwrap_err();
        assert!(err.to_string().contains("failed to read config file"));
    }

    #[test]
    fn config_load_fails_on_invalid_toml() {
        use tempfile::TempDir;
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let path = config_path().unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not = [valid").unwrap();

        let err = Config::load().unwrap_err();
        assert!(err.to_string().contains("failed to parse config file"));
    }

    #[test]
    fn config_save_fails_when_shnote_home_is_a_file() {
        use tempfile::TempDir;

        let i18n = test_i18n();
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let home_marker = temp_dir.path().join(".shnote");
        fs::write(&home_marker, "not a dir").unwrap();

        let err = Config::default().save(&i18n).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_create_config_dir(&home_marker.display().to_string())));
    }

    #[test]
    fn config_save_fails_when_config_path_is_directory() {
        use tempfile::TempDir;

        let i18n = test_i18n();
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let path = config_path().unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::create_dir_all(&path).unwrap();

        let err = Config::default().save(&i18n).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_write_config(&path.display().to_string())));
    }

    #[test]
    fn shnote_home_path_structure() {
        let shnote_home = shnote_home().unwrap();
        assert!(shnote_home.ends_with(".shnote"));
    }

    #[test]
    fn home_dir_returns_path() {
        let home = home_dir().unwrap();
        assert!(home.is_absolute());
    }

    #[test]
    fn home_dir_falls_back_to_userprofile() {
        use tempfile::TempDir;
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::set("USERPROFILE", temp_dir.path());

        assert_eq!(home_dir().unwrap(), PathBuf::from(temp_dir.path()));
    }

    #[test]
    fn home_dir_errors_when_missing_env_vars() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let err = home_dir().unwrap_err();
        assert!(err.to_string().contains("failed to determine home directory"));
    }

    #[test]
    fn config_load_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let err = Config::load().unwrap_err();
        assert!(err.to_string().contains("failed to determine home directory"));
    }

    #[test]
    fn config_save_errors_when_home_dir_missing() {
        let i18n = test_i18n();
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let err = Config::default().save(&i18n).unwrap_err();
        assert!(err.to_string().contains("failed to determine home directory"));
    }
}
