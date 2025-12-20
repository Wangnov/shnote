use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::i18n::I18n;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub paths: PathsConfig,

    #[serde(default)]
    pub i18n: I18nConfig,

    /// Output mode: default | quiet
    #[serde(default = "Config::default_output")]
    pub output: String,

    /// Colorize WHAT/WHY header output
    #[serde(default = "Config::default_color")]
    pub color: bool,

    /// Color for WHAT label
    #[serde(default = "Config::default_what_color")]
    pub what_color: String,

    /// Color for WHY label
    #[serde(default = "Config::default_why_color")]
    pub why_color: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: PathsConfig::default(),
            i18n: I18nConfig::default(),
            output: Self::default_output(),
            color: Self::default_color(),
            what_color: Self::default_what_color(),
            why_color: Self::default_why_color(),
        }
    }
}

const VALID_COLOR_NAMES: [&str; 17] = [
    "default",
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "bright_black",
    "bright_red",
    "bright_green",
    "bright_yellow",
    "bright_blue",
    "bright_magenta",
    "bright_cyan",
    "bright_white",
];

fn is_valid_color_name(name: &str) -> bool {
    VALID_COLOR_NAMES.contains(&name)
}

fn color_escape(name: &str, fallback: &'static str) -> Option<&'static str> {
    match name {
        "default" => None,
        "black" => Some("30"),
        "red" => Some("31"),
        "green" => Some("32"),
        "yellow" => Some("33"),
        "blue" => Some("34"),
        "magenta" => Some("35"),
        "cyan" => Some("36"),
        "white" => Some("37"),
        "bright_black" => Some("90"),
        "bright_red" => Some("91"),
        "bright_green" => Some("92"),
        "bright_yellow" => Some("93"),
        "bright_blue" => Some("94"),
        "bright_magenta" => Some("95"),
        "bright_cyan" => Some("96"),
        "bright_white" => Some("97"),
        _ => Some(fallback),
    }
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
    fn default_output() -> String {
        "default".to_string()
    }

    fn default_color() -> bool {
        true
    }

    fn default_what_color() -> String {
        "cyan".to_string()
    }

    fn default_why_color() -> String {
        "magenta".to_string()
    }

    /// Check if WHAT/WHY header should be printed
    pub fn should_print_header(&self) -> bool {
        self.output != "quiet"
    }

    /// Check if WHAT/WHY header should be colorized
    pub fn should_color_header(&self) -> bool {
        self.color
    }

    pub fn what_color_escape(&self) -> Option<&'static str> {
        color_escape(self.what_color.as_str(), "36")
    }

    pub fn why_color_escape(&self) -> Option<&'static str> {
        color_escape(self.why_color.as_str(), "35")
    }

    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Config::default());
        }
        let contents = fs::read_to_string(&path)
            .context(format!("failed to read config file: {}", path.display()))?;
        toml::from_str(&contents)
            .context(format!("failed to parse config file: {}", path.display()))
    }

    pub fn save(&self, i18n: &I18n) -> Result<()> {
        let parent = shnote_home()?;
        let path = parent.join("config.toml");
        fs::create_dir_all(&parent)
            .context(i18n.err_create_config_dir(&parent.display().to_string()))?;
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
            "output" => Some(self.output.clone()),
            "color" => Some(self.color.to_string()),
            "what_color" => Some(self.what_color.clone()),
            "why_color" => Some(self.why_color.clone()),
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
            "output" => {
                let valid = ["default", "quiet"];
                if !valid.contains(&value) {
                    anyhow::bail!(
                        "{}",
                        i18n.err_invalid_output_value(value, &valid.join(", "))
                    );
                }
                self.output = value.to_string();
                Ok(true)
            }
            "color" => {
                let normalized = value.to_lowercase();
                let parsed = match normalized.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => {
                        let valid = ["true", "false"];
                        anyhow::bail!("{}", i18n.err_invalid_color_value(value, &valid.join(", ")));
                    }
                };
                self.color = parsed;
                Ok(true)
            }
            "what_color" => {
                let normalized = value.to_lowercase();
                if !is_valid_color_name(&normalized) {
                    anyhow::bail!(
                        "{}",
                        i18n.err_invalid_color_name(value, &VALID_COLOR_NAMES.join(", "))
                    );
                }
                self.what_color = normalized;
                Ok(true)
            }
            "why_color" => {
                let normalized = value.to_lowercase();
                if !is_valid_color_name(&normalized) {
                    anyhow::bail!(
                        "{}",
                        i18n.err_invalid_color_name(value, &VALID_COLOR_NAMES.join(", "))
                    );
                }
                self.why_color = normalized;
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
            ("output".to_string(), self.output.clone()),
            ("color".to_string(), self.color.to_string()),
            ("what_color".to_string(), self.what_color.clone()),
            ("why_color".to_string(), self.why_color.clone()),
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
        assert_eq!(config.output, "default");
        assert!(config.color);
        assert_eq!(config.what_color, "cyan");
        assert_eq!(config.why_color, "magenta");
    }

    #[test]
    fn config_get_set() {
        let i18n = test_i18n();
        let mut config = Config::default();

        assert_eq!(config.get("python"), Some("python3".to_string()));
        assert_eq!(config.get("shell"), Some("auto".to_string()));
        assert_eq!(config.get("language"), Some("auto".to_string()));
        assert_eq!(config.get("output"), Some("default".to_string()));
        assert_eq!(config.get("color"), Some("true".to_string()));
        assert_eq!(config.get("what_color"), Some("cyan".to_string()));
        assert_eq!(config.get("why_color"), Some("magenta".to_string()));

        config.set(&i18n, "python", "/usr/bin/python3").unwrap();
        assert_eq!(config.get("python"), Some("/usr/bin/python3".to_string()));

        config.set(&i18n, "node", "/usr/bin/node").unwrap();
        assert_eq!(config.get("node"), Some("/usr/bin/node".to_string()));

        config.set(&i18n, "output", "quiet").unwrap();
        assert_eq!(config.get("output"), Some("quiet".to_string()));

        config.set(&i18n, "color", "false").unwrap();
        assert_eq!(config.get("color"), Some("false".to_string()));

        config.set(&i18n, "what_color", "yellow").unwrap();
        assert_eq!(config.get("what_color"), Some("yellow".to_string()));

        config.set(&i18n, "why_color", "blue").unwrap();
        assert_eq!(config.get("why_color"), Some("blue".to_string()));

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
    fn config_set_validates_output() {
        let i18n = test_i18n();
        let mut config = Config::default();

        assert!(config.set(&i18n, "output", "default").is_ok());
        assert!(config.set(&i18n, "output", "quiet").is_ok());
        assert!(config.set(&i18n, "output", "invalid").is_err());
    }

    #[test]
    fn config_set_validates_color() {
        let i18n = test_i18n();
        let mut config = Config::default();

        assert!(config.set(&i18n, "color", "true").is_ok());
        assert!(config.set(&i18n, "color", "false").is_ok());
        assert!(config.set(&i18n, "color", "invalid").is_err());
    }

    #[test]
    fn config_set_validates_label_colors() {
        let i18n = test_i18n();
        let mut config = Config::default();

        assert!(config.set(&i18n, "what_color", "cyan").is_ok());
        assert!(config.set(&i18n, "why_color", "magenta").is_ok());
        assert!(config.set(&i18n, "what_color", "bright_red").is_ok());
        assert!(config.set(&i18n, "why_color", "default").is_ok());
        assert!(config.set(&i18n, "what_color", "invalid").is_err());
    }

    #[test]
    fn color_escape_mapping() {
        let mut config = Config::default();
        assert_eq!(config.what_color_escape(), Some("36"));
        assert_eq!(config.why_color_escape(), Some("35"));

        config.what_color = "default".to_string();
        config.why_color = "bright_red".to_string();
        assert_eq!(config.what_color_escape(), None);
        assert_eq!(config.why_color_escape(), Some("91"));
    }

    #[test]
    fn should_print_header_default_is_true() {
        let config = Config::default();
        assert!(config.should_print_header());
    }

    #[test]
    fn should_print_header_quiet_is_false() {
        let config = Config {
            output: "quiet".to_string(),
            ..Default::default()
        };
        assert!(!config.should_print_header());
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
        assert_eq!(list.len(), 8);
        assert!(list.contains(&("python".to_string(), "python3".to_string())));
        assert!(list.contains(&("node".to_string(), "node".to_string())));
        assert!(list.contains(&("output".to_string(), "default".to_string())));
        assert!(list.contains(&("color".to_string(), "true".to_string())));
        assert!(list.contains(&("what_color".to_string(), "cyan".to_string())));
        assert!(list.contains(&("why_color".to_string(), "magenta".to_string())));
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
        assert!(err
            .to_string()
            .contains("failed to determine home directory"));
    }

    #[test]
    fn config_load_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let err = Config::load().unwrap_err();
        assert!(err
            .to_string()
            .contains("failed to determine home directory"));
    }

    #[test]
    fn config_save_errors_when_home_dir_missing() {
        let i18n = test_i18n();
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let err = Config::default().save(&i18n).unwrap_err();
        assert!(err
            .to_string()
            .contains("failed to determine home directory"));
    }
}
