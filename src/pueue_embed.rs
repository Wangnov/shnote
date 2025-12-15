use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::config::{pueue_binary_name, pueued_binary_name, shnote_bin_dir};
use crate::i18n::I18n;

/// Embedded pueue version
pub const PUEUE_VERSION: &str = "4.0.1";

/// SHA256 checksums for pueue binaries (v4.0.1)
pub(crate) mod checksums {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub(crate) const PUEUE_SHA256: &str =
        "4306f593b6a6b6db9d641889e33fe3a2effa6423888b8f82391fa57951ef1a9b";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub(crate) const PUEUED_SHA256: &str =
        "dc14a7873a4a474ae42e7a6ee5778c2af2d53049182ecaa2d061f4803f04bf23";

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    pub(crate) const PUEUE_SHA256: &str =
        "25f07f7e93f916d6189acc11846aab6ebee975b0cc5867cf40a96b5c70f3b55c";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    pub(crate) const PUEUED_SHA256: &str =
        "3e50d3bfadd1e417c8561aed2c1f4371605e8002f7fd793f39045719af5436a8";

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(crate) const PUEUE_SHA256: &str =
        "16aea6654b3915c6495bb2f456184fd7f3d418de3f74afb5eab04ae953cdfedf";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(crate) const PUEUED_SHA256: &str =
        "8a97b176f55929e37cda49577b28b66ea345151adf766b9d8efa8c9d81525a0b";

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    pub(crate) const PUEUE_SHA256: &str =
        "666af79b5a0246efa61a8589e51a190e3174bf80ad1c78b264204e7d312d43a9";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    pub(crate) const PUEUED_SHA256: &str =
        "8d3811f2ad57ef72ed171f446f19676ef755e189286d1c31a1e478ed57465bdb";

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    pub(crate) const PUEUE_SHA256: &str =
        "1ac310e87cf2333a5852cecb9519c4b8f07ec0701c81aff3a82638dd0202c65c";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    pub(crate) const PUEUED_SHA256: &str =
        "de1274b4d369f31efa1df0a75eb810954666a87109f6a2594a1b777517740601";

    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    pub(crate) const PUEUE_SHA256: &str = "";
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    pub(crate) const PUEUED_SHA256: &str = "";
}

/// Platform-specific binary data
///
/// To embed pueue binaries, download them from:
/// https://github.com/Nukesor/pueue/releases/tag/v4.0.1
///
/// Then place them in the assets/ directory and uncomment the include_bytes! lines below.
///
/// For development/testing, you can also use the setup command to download binaries
/// from the internet instead.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod embedded {
    // pub const PUEUE: &[u8] = include_bytes!("../assets/pueue-aarch64-apple-darwin");
    // pub const PUEUED: &[u8] = include_bytes!("../assets/pueued-aarch64-apple-darwin");
    pub const PUEUE: Option<&[u8]> = None;
    pub const PUEUED: Option<&[u8]> = None;
    pub const PLATFORM: &str = "aarch64-apple-darwin";
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
mod embedded {
    pub const PUEUE: Option<&[u8]> = None;
    pub const PUEUED: Option<&[u8]> = None;
    pub const PLATFORM: &str = "x86_64-apple-darwin";
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod embedded {
    pub const PUEUE: Option<&[u8]> = None;
    pub const PUEUED: Option<&[u8]> = None;
    pub const PLATFORM: &str = "x86_64-unknown-linux-musl";
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
mod embedded {
    pub const PUEUE: Option<&[u8]> = None;
    pub const PUEUED: Option<&[u8]> = None;
    pub const PLATFORM: &str = "aarch64-unknown-linux-musl";
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
mod embedded {
    pub const PUEUE: Option<&[u8]> = None;
    pub const PUEUED: Option<&[u8]> = None;
    pub const PLATFORM: &str = "x86_64-pc-windows-msvc";
}

#[cfg(not(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "macos", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64"),
)))]
mod embedded {
    pub const PUEUE: Option<&[u8]> = None;
    pub const PUEUED: Option<&[u8]> = None;
    pub const PLATFORM: &str = "unsupported";
}

pub fn run_setup(i18n: &I18n) -> Result<()> {
    let bin_dir = shnote_bin_dir()?;

    println!("{}", i18n.setup_starting());
    println!("  Platform: {}", embedded::PLATFORM);
    println!("  Target directory: {}", bin_dir.display());
    println!();

    // Create bin directory
    fs::create_dir_all(&bin_dir)
        .with_context(|| i18n.err_create_dir(&bin_dir.display().to_string()))?;

    install_binaries(i18n, &bin_dir, embedded::PUEUE, embedded::PUEUED)?;

    // Print PATH instructions
    println!();
    println!("{}", i18n.setup_path_instruction());
    println!();

    #[cfg(unix)]
    {
        println!("  # Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):");
        println!("  export PATH=\"{}:$PATH\"", bin_dir.display());
    }

    #[cfg(windows)]
    {
        println!("  # Add to your PATH environment variable:");
        println!("  {}", bin_dir.display());
    }

    println!();
    println!("{}", i18n.setup_complete());

    Ok(())
}

fn install_binaries(
    i18n: &I18n,
    bin_dir: &Path,
    pueue: Option<&[u8]>,
    pueued: Option<&[u8]>,
) -> Result<()> {
    match (pueue, pueued) {
        (Some(pueue), Some(pueued)) => extract_embedded_binaries(i18n, bin_dir, pueue, pueued),
        _ => download_binaries(i18n, bin_dir),
    }
}

fn extract_embedded_binaries(
    i18n: &I18n,
    bin_dir: &Path,
    pueue: &[u8],
    pueued: &[u8],
) -> Result<()> {
    println!("{}", i18n.setup_extracting());

    // Extract pueue
    let pueue_path = bin_dir.join(pueue_binary_name());
    write_binary(i18n, &pueue_path, pueue)?;
    println!("  ✓ pueue -> {}", pueue_path.display());

    // Extract pueued
    let pueued_path = bin_dir.join(pueued_binary_name());
    write_binary(i18n, &pueued_path, pueued)?;
    println!("  ✓ pueued -> {}", pueued_path.display());

    Ok(())
}

fn download_binaries(i18n: &I18n, bin_dir: &Path) -> Result<()> {
    println!("{}", i18n.setup_downloading());
    println!();

    let github_proxy = std::env::var("GITHUB_PROXY").ok();
    let base_url = format!(
        "https://github.com/Nukesor/pueue/releases/download/v{}/",
        PUEUE_VERSION
    );
    let base_url = apply_github_proxy(&github_proxy, &base_url);

    if let Some(proxy) = &github_proxy {
        println!("  Using GitHub proxy: {}", proxy);
        println!();
    }

    let (pueue_filename, pueued_filename) = get_release_filenames();

    println!("  Downloading pueue...");
    let pueue_url = format!("{}{}", base_url, pueue_filename);
    let pueue_path = bin_dir.join(pueue_binary_name());
    download_and_verify(i18n, &pueue_url, &pueue_path, checksums::PUEUE_SHA256)?;
    println!("  ✓ pueue -> {}", pueue_path.display());

    println!("  Downloading pueued...");
    let pueued_url = format!("{}{}", base_url, pueued_filename);
    let pueued_path = bin_dir.join(pueued_binary_name());
    download_and_verify(i18n, &pueued_url, &pueued_path, checksums::PUEUED_SHA256)?;
    println!("  ✓ pueued -> {}", pueued_path.display());

    Ok(())
}

/// Apply GitHub proxy prefix to URL if GITHUB_PROXY is set
fn apply_github_proxy(proxy: &Option<String>, url: &str) -> String {
    match proxy {
        Some(p) => {
            let proxy = p.trim_end_matches('/');
            format!("{}/{}", proxy, url)
        }
        None => url.to_string(),
    }
}

fn get_release_filenames() -> (&'static str, &'static str) {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        ("pueue-aarch64-apple-darwin", "pueued-aarch64-apple-darwin")
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        ("pueue-x86_64-apple-darwin", "pueued-x86_64-apple-darwin")
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        (
            "pueue-x86_64-unknown-linux-musl",
            "pueued-x86_64-unknown-linux-musl",
        )
    }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        (
            "pueue-aarch64-unknown-linux-musl",
            "pueued-aarch64-unknown-linux-musl",
        )
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        (
            "pueue-x86_64-pc-windows-msvc.exe",
            "pueued-x86_64-pc-windows-msvc.exe",
        )
    }

    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    {
        ("unsupported", "unsupported")
    }
}

fn download_and_verify(
    i18n: &I18n,
    url: &str,
    dest: &PathBuf,
    expected_sha256: &str,
) -> Result<()> {
    download_file(i18n, url, dest)?;

    // Verify SHA256 checksum
    if expected_sha256.is_empty() {
        return Ok(());
    }

    let actual_sha256 = compute_sha256(i18n, dest)?;
    if actual_sha256 != expected_sha256 {
        // Remove the corrupted file
        let _ = fs::remove_file(dest);
        anyhow::bail!(
            "{}",
            i18n.err_checksum_mismatch(
                &dest.display().to_string(),
                expected_sha256,
                &actual_sha256
            )
        );
    }

    Ok(())
}

fn compute_sha256(i18n: &I18n, path: &PathBuf) -> Result<String> {
    // Use shasum on Unix or certutil on Windows
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
        // certutil output format:
        // SHA256 hash of file:
        // <hash>
        // CertUtil: -hashfile command completed successfully.
        let hash = stdout
            .lines()
            .nth(1)
            .context(i18n.err_certutil_parse())?
            .trim()
            .to_lowercase();
        Ok(hash)
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
                // Make executable
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(0o755);
                fs::set_permissions(dest, perms)?;
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

        // Make executable
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(dest, perms)?;
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

fn write_binary(i18n: &I18n, path: &PathBuf, data: &[u8]) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| i18n.err_create_file(&path.display().to_string()))?;
    write_binary_with_writer(i18n, path, &mut file, data)
}

fn write_binary_with_writer(
    i18n: &I18n,
    path: &PathBuf,
    writer: &mut dyn Write,
    data: &[u8],
) -> Result<()> {
    write_binary_to_writer(i18n, path, writer, data)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    Ok(())
}

fn write_binary_to_writer(
    i18n: &I18n,
    path: &Path,
    writer: &mut dyn Write,
    data: &[u8],
) -> Result<()> {
    writer
        .write_all(data)
        .with_context(|| i18n.err_write_file(&path.display().to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::test_support::{env_lock, EnvVarGuard};
    use tempfile::TempDir;

    #[test]
    fn pueue_version_is_set() {
        assert!(PUEUE_VERSION.starts_with("4."));
        assert!(PUEUE_VERSION.len() >= 3); // At least "4.x"
    }

    #[test]
    fn embedded_platform_is_set() {
        assert_ne!(embedded::PLATFORM, "unsupported");
        assert!(embedded::PLATFORM.contains('-')); // e.g., "aarch64-apple-darwin"
    }

    #[test]
    fn get_release_filenames_returns_valid_names() {
        let (pueue, pueued) = get_release_filenames();
        assert!(pueue.contains("pueue"));
        assert!(pueued.contains("pueued"));
    }

    #[test]
    fn checksums_are_valid_sha256() {
        // SHA256 hashes are 64 hex characters
        assert_eq!(checksums::PUEUE_SHA256.len(), 64);
        assert_eq!(checksums::PUEUED_SHA256.len(), 64);
        assert!(checksums::PUEUE_SHA256
            .chars()
            .all(|c| c.is_ascii_hexdigit()));
        assert!(checksums::PUEUED_SHA256
            .chars()
            .all(|c| c.is_ascii_hexdigit()));
    }

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

    fn test_i18n() -> I18n {
        I18n::new(Lang::En)
    }

    #[cfg(unix)]
    fn make_fake_tools_dir() -> TempDir {
        TempDir::new().expect("temp dir")
    }

    #[cfg(unix)]
    fn setup_path_with(dir: &TempDir) -> EnvVarGuard {
        EnvVarGuard::set("PATH", dir.path())
    }

    #[cfg(unix)]
    fn write_tool(dir: &TempDir, name: &str, script: &str) {
        use crate::test_support::write_executable;
        let path = dir.path().join(name);
        write_executable(&path, script).expect("write fake tool");
    }

    #[test]
    fn install_binaries_uses_embedded_when_available() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let pueue = b"pueue-bytes";
        let pueued = b"pueued-bytes";

        install_binaries(&i18n, temp_dir.path(), Some(pueue), Some(pueued)).unwrap();

        let pueue_path = temp_dir.path().join(pueue_binary_name());
        let pueued_path = temp_dir.path().join(pueued_binary_name());

        assert_eq!(fs::read(&pueue_path).unwrap(), pueue);
        assert_eq!(fs::read(&pueued_path).unwrap(), pueued);
    }

    #[test]
    fn run_setup_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = test_i18n();
        let err = run_setup(&i18n).unwrap_err();
        assert!(err
            .to_string()
            .contains("failed to determine home directory"));
    }

    #[cfg(unix)]
    #[test]
    fn run_setup_errors_when_install_binaries_fails() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());

        let err = run_setup(&i18n).unwrap_err();
        assert!(err.to_string().contains(i18n.err_download_no_tool()));
    }

    #[test]
    fn extract_embedded_binaries_errors_when_first_write_fails() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let not_a_dir = temp_dir.path().join("not_a_dir");
        fs::write(&not_a_dir, "file").unwrap();

        let err = extract_embedded_binaries(&i18n, &not_a_dir, b"pueue", b"pueued").unwrap_err();
        assert!(err.to_string().contains(
            &i18n.err_create_file(&not_a_dir.join(pueue_binary_name()).display().to_string())
        ));
    }

    #[test]
    fn extract_embedded_binaries_errors_when_second_write_fails() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path();

        // Make pueued path a directory so writing pueued fails.
        let pueued_path = bin_dir.join(pueued_binary_name());
        fs::create_dir_all(&pueued_path).unwrap();

        let err = extract_embedded_binaries(&i18n, bin_dir, b"pueue", b"pueued").unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_create_file(&pueued_path.display().to_string())));
    }

    #[cfg(unix)]
    #[test]
    fn download_binaries_errors_when_first_binary_checksum_mismatch() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(
            &tools,
            "curl",
            r#"#!/bin/sh
# args: -fsSL -o DEST URL
printf "bin" > "$3"
exit 0
"#,
        );
        write_tool(
            &tools,
            "shasum",
            r#"#!/bin/sh
echo "wrong  $3"
exit 0
"#,
        );

        let bin_dir = TempDir::new().unwrap();
        let err = download_binaries(&i18n, bin_dir.path()).unwrap_err();
        assert!(err.to_string().contains("checksum"));
    }

    #[cfg(unix)]
    #[test]
    fn download_binaries_errors_when_second_binary_checksum_mismatch() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(
            &tools,
            "curl",
            r#"#!/bin/sh
# args: -fsSL -o DEST URL
printf "bin" > "$3"
exit 0
"#,
        );
        let pueue_hash = checksums::PUEUE_SHA256;
        write_tool(
            &tools,
            "shasum",
            &format!(
                "#!/bin/sh\n\
file=\"$3\"\n\
case \"$file\" in\n\
  *pueue) echo \"{pueue_hash}  $file\" ;;\n\
  *pueued) echo \"wrong  $file\" ;;\n\
  *) echo \"{pueue_hash}  $file\" ;;\n\
esac\n\
exit 0\n"
            ),
        );

        let bin_dir = TempDir::new().unwrap();
        let err = download_binaries(&i18n, bin_dir.path()).unwrap_err();
        assert!(err.to_string().contains("checksum"));
    }

    #[cfg(unix)]
    #[test]
    fn download_file_falls_back_to_wget_when_curl_fails() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(&tools, "curl", "#!/bin/sh\nexit 1\n");
        write_tool(
            &tools,
            "wget",
            r#"#!/bin/sh
# args: -q -O DEST URL
printf "bin" > "$3"
exit 0
"#,
        );

        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("pueue");
        download_file(&i18n, "https://example.invalid/pueue", &dest).unwrap();
        assert_eq!(fs::read_to_string(dest).unwrap(), "bin");
    }

    #[cfg(unix)]
    #[test]
    fn download_file_errors_when_curl_cannot_set_permissions() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(
            &tools,
            "curl",
            r#"#!/bin/sh
# args: -fsSL -o DEST URL
printf "bin" > "$3"
exit 0
"#,
        );

        let dest = PathBuf::from("/dev/null");
        assert!(download_file(&i18n, "https://example.invalid/pueue", &dest).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn download_file_errors_when_wget_cannot_set_permissions() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(&tools, "curl", "#!/bin/sh\nexit 1\n");
        write_tool(
            &tools,
            "wget",
            r#"#!/bin/sh
# args: -q -O DEST URL
printf "bin" > "$3"
exit 0
"#,
        );

        let dest = PathBuf::from("/dev/null");
        assert!(download_file(&i18n, "https://example.invalid/pueue", &dest).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn download_file_errors_when_wget_missing() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(&tools, "curl", "#!/bin/sh\nexit 1\n");

        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("pueue");
        let err = download_file(&i18n, "https://example.invalid/pueue", &dest).unwrap_err();
        assert!(err.to_string().contains(i18n.err_download_no_tool()));
    }

    #[cfg(unix)]
    #[test]
    fn download_file_errors_when_wget_fails() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(&tools, "curl", "#!/bin/sh\nexit 1\n");
        write_tool(&tools, "wget", "#!/bin/sh\nexit 2\n");

        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("pueue");
        let err = download_file(&i18n, "https://example.invalid/pueue", &dest).unwrap_err();
        assert!(err.to_string().contains(i18n.err_download_failed()));
    }

    #[cfg(unix)]
    #[test]
    fn download_and_verify_removes_file_on_checksum_mismatch() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(
            &tools,
            "curl",
            r#"#!/bin/sh
# args: -fsSL -o DEST URL
printf "downloaded" > "$3"
exit 0
"#,
        );
        write_tool(
            &tools,
            "shasum",
            r#"#!/bin/sh
echo "actualhash  $3"
exit 0
"#,
        );

        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("pueue");
        let err = download_and_verify(
            &i18n,
            "https://example.invalid/pueue",
            &dest,
            "expectedhash",
        )
        .unwrap_err();

        assert!(!dest.exists());
        assert!(err.to_string().contains("checksum"));
    }

    #[cfg(unix)]
    #[test]
    fn download_and_verify_errors_when_download_fails() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(&tools, "curl", "#!/bin/sh\nexit 1\n");
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("pueue");
        let err =
            download_and_verify(&i18n, "https://example.invalid/pueue", &dest, "").unwrap_err();
        assert!(err.to_string().contains(i18n.err_download_no_tool()));
    }

    #[cfg(unix)]
    #[test]
    fn download_and_verify_errors_when_checksum_tool_missing() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(
            &tools,
            "curl",
            r#"#!/bin/sh
# args: -fsSL -o DEST URL
printf "downloaded" > "$3"
exit 0
"#,
        );

        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("pueue");
        let err = download_and_verify(&i18n, "https://example.invalid/pueue", &dest, "expected")
            .unwrap_err();
        assert!(err.to_string().contains(i18n.err_shasum_run()));
    }

    #[cfg(unix)]
    #[test]
    fn download_and_verify_skips_checksum_when_expected_empty() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(
            &tools,
            "curl",
            r#"#!/bin/sh
# args: -fsSL -o DEST URL
printf "downloaded" > "$3"
exit 0
"#,
        );

        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("pueue");
        download_and_verify(&i18n, "https://example.invalid/pueue", &dest, "").unwrap();
        assert_eq!(fs::read_to_string(dest).unwrap(), "downloaded");
    }

    #[cfg(unix)]
    #[test]
    fn compute_sha256_errors_when_shasum_missing() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("x");
        fs::write(&file, "data").unwrap();

        let err = compute_sha256(&i18n, &file).unwrap_err();
        assert!(err.to_string().contains(i18n.err_shasum_run()));
    }

    #[cfg(unix)]
    #[test]
    fn compute_sha256_errors_when_shasum_fails() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(&tools, "shasum", "#!/bin/sh\nexit 1\n");

        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("x");
        fs::write(&file, "data").unwrap();

        let err = compute_sha256(&i18n, &file).unwrap_err();
        assert!(err.to_string().contains(i18n.err_shasum_failed()));
    }

    #[cfg(unix)]
    #[test]
    fn compute_sha256_errors_when_shasum_output_unparseable() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let tools = make_fake_tools_dir();
        let _path_guard = setup_path_with(&tools);

        write_tool(&tools, "shasum", "#!/bin/sh\nexit 0\n");

        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("x");
        fs::write(&file, "data").unwrap();

        let err = compute_sha256(&i18n, &file).unwrap_err();
        assert!(err.to_string().contains(i18n.err_shasum_parse()));
    }

    #[test]
    fn write_binary_errors_when_parent_missing() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let missing_parent = temp_dir.path().join("missing").join("pueue");
        let err = write_binary(&i18n, &missing_parent, b"data").unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_create_file(&missing_parent.display().to_string())));
    }

    #[cfg(unix)]
    #[test]
    fn write_binary_errors_when_cannot_set_permissions() {
        let i18n = test_i18n();
        let path = PathBuf::from("/dev/null");
        assert!(write_binary(&i18n, &path, b"data").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn write_binary_errors_when_write_fails() {
        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("boom"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let i18n = test_i18n();
        let path = PathBuf::from("dummy");
        let mut writer = FailingWriter;
        let err = write_binary_with_writer(&i18n, &path, &mut writer, b"data").unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_write_file(&path.display().to_string())));
        assert!(writer.flush().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn run_setup_errors_when_bin_dir_cannot_be_created() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Make ~/.shnote a file so ~/.shnote/bin cannot be created.
        let shnote_home = temp_dir.path().join(".shnote");
        fs::write(&shnote_home, "not a dir").unwrap();

        let err = run_setup(&i18n).unwrap_err();
        let expected =
            i18n.err_create_dir(&temp_dir.path().join(".shnote/bin").display().to_string());
        assert!(err.to_string().contains(&expected));
    }
}
