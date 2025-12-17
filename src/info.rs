use std::path::PathBuf;

use anyhow::Result;

use crate::config::{
    home_dir, pueue_binary_name, pueued_binary_name, shnote_bin_dir, shnote_home,
};
use crate::i18n::I18n;
use crate::pueue_embed::{embedded, PUEUE_VERSION};

/// Current shnote version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Platform target triple
pub const PLATFORM: &str = embedded::PLATFORM;

/// GitHub repository
pub const REPO: &str = "wangnov/shnote";

pub fn run_info(i18n: &I18n) -> Result<()> {
    // Version and platform
    println!("shnote {} ({})", VERSION, PLATFORM);
    println!();

    // Paths
    let install_path = get_install_path();
    let config_path = shnote_home().ok().map(|p| p.join("config.toml"));
    let data_path = shnote_home().ok();

    println!("{}:", i18n.info_paths());
    println!(
        "  {}: {}",
        i18n.info_install_path(),
        install_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| i18n.info_unknown().to_string())
    );
    println!(
        "  {}: {}",
        i18n.info_config_path(),
        config_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| i18n.info_unknown().to_string())
    );
    println!(
        "  {}: {}",
        i18n.info_data_path(),
        data_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| i18n.info_unknown().to_string())
    );
    println!();

    // Components
    println!("{}:", i18n.info_components());

    let bin_dir = shnote_bin_dir().ok();
    let pueue_path = bin_dir.as_ref().map(|d| d.join(pueue_binary_name()));
    let pueued_path = bin_dir.as_ref().map(|d| d.join(pueued_binary_name()));

    let pueue_installed = pueue_path
        .as_ref()
        .map(|p| p.exists())
        .unwrap_or(false);
    let pueued_installed = pueued_path
        .as_ref()
        .map(|p| p.exists())
        .unwrap_or(false);

    if pueue_installed && pueued_installed {
        println!(
            "  pueue   v{}  {}",
            PUEUE_VERSION,
            i18n.info_installed()
        );
        println!(
            "  pueued  v{}  {}",
            PUEUE_VERSION,
            i18n.info_installed()
        );
    } else {
        println!("  pueue   {}  {}", i18n.info_not_installed(), i18n.info_run_setup());
        println!("  pueued  {}  {}", i18n.info_not_installed(), i18n.info_run_setup());
    }

    Ok(())
}

/// Get the path to the currently running shnote executable
pub fn get_install_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

/// Get the default install directory based on platform
#[allow(dead_code)]
pub fn get_default_install_dir() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        home_dir().ok().map(|h| h.join(".local/bin"))
    }
    #[cfg(windows)]
    {
        home_dir().ok().map(|h| h.join(".local\\bin"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.contains('.'));
    }

    #[test]
    fn platform_is_set() {
        assert!(!PLATFORM.is_empty());
        assert!(PLATFORM.contains('-'));
    }

    #[test]
    fn repo_is_set() {
        assert_eq!(REPO, "wangnov/shnote");
    }

    #[test]
    fn get_install_path_returns_some() {
        // When running tests, we should be able to get the test binary path
        assert!(get_install_path().is_some());
    }

    #[test]
    fn get_default_install_dir_returns_some() {
        use crate::test_support::{env_lock, EnvVarGuard};
        use tempfile::TempDir;

        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let dir = get_default_install_dir();
        assert!(dir.is_some());
        let dir = dir.unwrap();
        assert!(dir.ends_with(".local/bin") || dir.ends_with(".local\\bin"));
    }
}
