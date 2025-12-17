use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::cli::UpdateArgs;
use crate::i18n::I18n;
use crate::info::{get_install_path, PLATFORM, REPO, VERSION};

/// URL pattern for VERSION file
const VERSION_URL: &str = "https://github.com/{repo}/releases/latest/download/VERSION";

/// URL pattern for binary download
const BINARY_URL: &str = "https://github.com/{repo}/releases/download/{version}/shnote-{platform}";

/// URL pattern for checksum download
const CHECKSUM_URL: &str =
    "https://github.com/{repo}/releases/download/{version}/shnote-{platform}.sha256";

pub fn run_update(i18n: &I18n, args: UpdateArgs) -> Result<()> {
    println!("{}", i18n.update_checking());

    // Get current version
    let current_version = VERSION;
    println!("  {}: v{}", i18n.update_current_version(), current_version);

    // Fetch latest version
    let latest_version = fetch_latest_version(i18n)?;
    let latest_version = latest_version.trim().trim_start_matches('v');
    println!("  {}: v{}", i18n.update_latest_version(), latest_version);
    println!();

    // Compare versions
    if current_version == latest_version && !args.force {
        println!("{}", i18n.update_already_latest());
        return Ok(());
    }

    if args.check {
        if current_version != latest_version {
            println!("{}", i18n.update_available(&format!("v{}", latest_version)));
        }
        return Ok(());
    }

    // Download and install
    println!(
        "{}",
        i18n.update_downloading(&format!("v{}", latest_version))
    );

    let install_path = get_install_path().context(i18n.update_err_install_path())?;

    download_and_install(i18n, latest_version, &install_path)?;

    println!();
    println!("{}", i18n.update_success(&format!("v{}", latest_version)));

    Ok(())
}

fn fetch_latest_version(i18n: &I18n) -> Result<String> {
    let github_proxy = env::var("GITHUB_PROXY").ok();
    let url = VERSION_URL.replace("{repo}", REPO);
    let url = apply_github_proxy(&github_proxy, &url);

    if let Some(proxy) = &github_proxy {
        println!("  {}: {}", i18n.update_using_proxy(), proxy);
    }

    let temp_dir = tempfile::tempdir().context(i18n.update_err_temp_dir())?;
    let version_file = temp_dir.path().join("VERSION");

    download_file(i18n, &url, &version_file)?;

    let content = fs::read_to_string(&version_file).context(i18n.update_err_read_version())?;

    Ok(content)
}

fn download_and_install(i18n: &I18n, version: &str, install_path: &PathBuf) -> Result<()> {
    let github_proxy = env::var("GITHUB_PROXY").ok();

    // Build download URLs
    let version_tag = format!("v{}", version);
    let binary_name = get_binary_name();

    let binary_url = BINARY_URL
        .replace("{repo}", REPO)
        .replace("{version}", &version_tag)
        .replace("{platform}", PLATFORM);
    let binary_url = apply_github_proxy(&github_proxy, &binary_url);

    let checksum_url = CHECKSUM_URL
        .replace("{repo}", REPO)
        .replace("{version}", &version_tag)
        .replace("{platform}", PLATFORM);
    let checksum_url = apply_github_proxy(&github_proxy, &checksum_url);

    // Create temp directory
    let temp_dir = tempfile::tempdir().context(i18n.update_err_temp_dir())?;
    let temp_binary = temp_dir.path().join(&binary_name);
    let temp_checksum = temp_dir.path().join(format!("{}.sha256", binary_name));

    // Download binary and checksum
    download_file(i18n, &binary_url, &temp_binary)?;
    download_file(i18n, &checksum_url, &temp_checksum)?;

    // Verify checksum
    println!("  {}", i18n.update_verifying());
    let expected_hash = read_checksum_file(&temp_checksum)?;
    let actual_hash = compute_sha256(i18n, &temp_binary)?;

    if actual_hash != expected_hash {
        anyhow::bail!(
            "{}",
            i18n.err_checksum_mismatch(
                &temp_binary.display().to_string(),
                &expected_hash,
                &actual_hash
            )
        );
    }

    // Replace binary
    println!("  {}", i18n.update_installing());
    replace_binary(i18n, &temp_binary, install_path)?;

    Ok(())
}

fn get_binary_name() -> String {
    #[cfg(windows)]
    {
        format!("shnote-{}.exe", PLATFORM)
    }
    #[cfg(not(windows))]
    {
        format!("shnote-{}", PLATFORM)
    }
}

fn apply_github_proxy(proxy: &Option<String>, url: &str) -> String {
    match proxy {
        Some(p) => {
            let proxy = p.trim_end_matches('/');
            format!("{}/{}", proxy, url)
        }
        None => url.to_string(),
    }
}

fn download_file(i18n: &I18n, url: &str, dest: &PathBuf) -> Result<()> {
    #[cfg(unix)]
    {
        // Try curl first
        let status = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(dest)
            .arg(url)
            .stderr(Stdio::inherit())
            .status();

        match status {
            Ok(s) if s.success() => {
                return Ok(());
            }
            _ => {}
        }

        // Try wget as fallback
        let status = Command::new("wget")
            .args(["-q", "-O"])
            .arg(dest)
            .arg(url)
            .status()
            .context(i18n.err_download_no_tool())?;

        if !status.success() {
            anyhow::bail!("{}", i18n.err_download_failed());
        }
    }

    #[cfg(windows)]
    {
        // Use PowerShell to download
        let script = format!(
            "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
            url,
            dest.display()
        );

        let status = Command::new("powershell")
            .args(["-Command", &script])
            .status()
            .context(i18n.err_download_powershell())?;

        if !status.success() {
            anyhow::bail!("{}", i18n.err_download_failed());
        }
    }

    Ok(())
}

fn read_checksum_file(path: &PathBuf) -> Result<String> {
    let content = fs::read_to_string(path)?;
    // Format: "hash  filename" or just "hash"
    Ok(content
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase())
}

fn compute_sha256(i18n: &I18n, path: &PathBuf) -> Result<String> {
    #[cfg(unix)]
    {
        let output = Command::new("shasum")
            .args(["-a", "256"])
            .arg(path)
            .output()
            .context(i18n.err_shasum_run())?;

        if !output.status.success() {
            anyhow::bail!("{}", i18n.err_shasum_failed());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let hash = stdout
            .split_whitespace()
            .next()
            .context(i18n.err_shasum_parse())?;
        Ok(hash.to_string())
    }

    #[cfg(windows)]
    {
        let output = Command::new("certutil")
            .args(["-hashfile"])
            .arg(path)
            .arg("SHA256")
            .output()
            .context(i18n.err_certutil_run())?;

        if !output.status.success() {
            anyhow::bail!("{}", i18n.err_certutil_failed());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let hash = stdout
            .lines()
            .nth(1)
            .context(i18n.err_certutil_parse())?
            .trim()
            .to_lowercase();
        Ok(hash)
    }
}

fn replace_binary(i18n: &I18n, src: &PathBuf, dest: &PathBuf) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // Make executable
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(src, perms)?;

        // On Unix, we can replace a running binary
        fs::copy(src, dest).context(i18n.update_err_replace_binary())?;
    }

    #[cfg(windows)]
    {
        // On Windows, we need to rename the running binary first
        let dest_old = dest.with_extension("exe.old");

        // Remove old backup if exists
        let _ = fs::remove_file(&dest_old);

        // Rename current binary to .old
        if dest.exists() {
            fs::rename(dest, &dest_old).context(i18n.update_err_rename_old())?;
        }

        // Copy new binary
        fs::copy(src, dest).context(i18n.update_err_replace_binary())?;

        // Try to remove old binary (may fail if still in use)
        let _ = fs::remove_file(&dest_old);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_github_proxy_without_proxy() {
        let url = "https://github.com/example/file";
        assert_eq!(apply_github_proxy(&None, url), url);
    }

    #[test]
    fn apply_github_proxy_with_proxy() {
        let proxy = Some("https://ghfast.top".to_string());
        let url = "https://github.com/example/file";
        assert_eq!(
            apply_github_proxy(&proxy, url),
            "https://ghfast.top/https://github.com/example/file"
        );
    }

    #[test]
    fn apply_github_proxy_strips_trailing_slash() {
        let proxy = Some("https://ghfast.top/".to_string());
        let url = "https://github.com/example/file";
        assert_eq!(
            apply_github_proxy(&proxy, url),
            "https://ghfast.top/https://github.com/example/file"
        );
    }

    #[test]
    fn get_binary_name_is_platform_specific() {
        let name = get_binary_name();
        assert!(name.starts_with("shnote-"));
        #[cfg(windows)]
        assert!(name.ends_with(".exe"));
        #[cfg(not(windows))]
        assert!(!name.ends_with(".exe"));
    }

    #[test]
    fn read_checksum_file_parses_hash() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let checksum_file = temp_dir.path().join("test.sha256");

        // Test "hash  filename" format
        fs::write(&checksum_file, "abc123def456  shnote-platform").unwrap();
        assert_eq!(read_checksum_file(&checksum_file).unwrap(), "abc123def456");

        // Test "hash" only format
        fs::write(&checksum_file, "ABC123DEF456").unwrap();
        assert_eq!(read_checksum_file(&checksum_file).unwrap(), "abc123def456");
    }
}
