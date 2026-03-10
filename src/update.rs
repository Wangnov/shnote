use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};

use crate::cli::{InitTarget, UpdateArgs};
use crate::config::home_dir;
use crate::i18n::I18n;
use crate::info::{get_install_path, PLATFORM, REPO, VERSION};
use crate::init::{rules_for_target_with_pueue, SHNOTE_MARKER_END, SHNOTE_MARKER_START};

/// URL pattern for cargo-dist manifest
const DIST_MANIFEST_URL: &str =
    "https://github.com/{repo}/releases/latest/download/dist-manifest.json";

#[derive(Debug, Deserialize)]
struct DistManifest {
    announcement_tag: String,
    #[serde(default, deserialize_with = "deserialize_artifacts")]
    artifacts: Vec<DistArtifact>,
}

#[derive(Debug, Deserialize)]
struct DistArtifact {
    name: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    target_triples: Vec<String>,
    #[serde(default)]
    checksums: DistChecksums,
    #[serde(default)]
    assets: Vec<DistAsset>,
}

#[derive(Debug, Default, Deserialize)]
struct DistChecksums {
    #[serde(default)]
    sha256: String,
}

#[derive(Debug, Default, Deserialize)]
struct DistAsset {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    path: String,
}

#[derive(Debug, Clone)]
struct LatestRelease {
    version: String,
    tag: String,
    archive_name: String,
    archive_sha256: String,
    executable_path: String,
}

fn deserialize_artifacts<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<DistArtifact>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ArtifactsRepr {
        List(Vec<DistArtifact>),
        Map(BTreeMap<String, DistArtifact>),
    }

    match ArtifactsRepr::deserialize(deserializer)? {
        ArtifactsRepr::List(artifacts) => Ok(artifacts),
        ArtifactsRepr::Map(artifacts) => Ok(artifacts.into_values().collect()),
    }
}

pub fn run_update(i18n: &I18n, args: UpdateArgs) -> Result<()> {
    println!("{}", i18n.update_checking());

    // Get current version
    let current_version = VERSION;
    println!("  {}: v{}", i18n.update_current_version(), current_version);

    // Fetch latest release metadata
    let latest_release = fetch_latest_release(i18n)?;
    println!(
        "  {}: v{}",
        i18n.update_latest_version(),
        latest_release.version
    );
    println!();

    // Compare versions
    if current_version == latest_release.version && !args.force {
        println!("{}", i18n.update_already_latest());
        return Ok(());
    }

    if args.check {
        if current_version != latest_release.version {
            println!(
                "{}",
                i18n.update_available(&format!("v{}", latest_release.version))
            );
        }
        return Ok(());
    }

    // Download and install
    println!(
        "{}",
        i18n.update_downloading(&format!("v{}", latest_release.version))
    );

    let install_path = get_install_path().context(i18n.update_err_install_path())?;

    download_and_install(i18n, &latest_release, &install_path)?;

    println!();
    println!(
        "{}",
        i18n.update_success(&format!("v{}", latest_release.version))
    );
    println!();

    check_rules_after_update(i18n, &install_path)?;

    Ok(())
}

fn fetch_latest_release(i18n: &I18n) -> Result<LatestRelease> {
    let github_proxy = env::var("GITHUB_PROXY").ok();
    let url = DIST_MANIFEST_URL.replace("{repo}", REPO);
    let url = apply_github_proxy(&github_proxy, &url);

    if let Some(proxy) = &github_proxy {
        println!("  {}: {}", i18n.update_using_proxy(), proxy);
    }

    let temp_dir = tempfile::tempdir().context(i18n.update_err_temp_dir())?;
    let manifest_file = temp_dir.path().join("dist-manifest.json");

    download_file(i18n, &url, &manifest_file)?;

    let content = fs::read_to_string(&manifest_file).context(i18n.update_err_read_version())?;

    latest_release_from_manifest(&content, PLATFORM, i18n)
}

fn parse_dist_manifest(json: &str, i18n: &I18n) -> Result<DistManifest> {
    serde_json::from_str(json).context(i18n.update_err_parse_manifest())
}

fn latest_release_from_manifest(json: &str, platform: &str, i18n: &I18n) -> Result<LatestRelease> {
    let manifest = parse_dist_manifest(json, i18n)?;
    let artifact = select_platform_artifact(&manifest, platform, i18n)?;
    let executable_path = artifact_executable_path(artifact, i18n)?;
    let tag = manifest.announcement_tag.trim().to_string();
    let version = tag.trim_start_matches('v').to_string();

    Ok(LatestRelease {
        version,
        tag,
        archive_name: artifact.name.clone(),
        archive_sha256: artifact.checksums.sha256.clone(),
        executable_path: executable_path.to_string(),
    })
}

fn select_platform_artifact<'a>(
    manifest: &'a DistManifest,
    platform: &str,
    i18n: &I18n,
) -> Result<&'a DistArtifact> {
    manifest
        .artifacts
        .iter()
        .find(|artifact| {
            artifact
                .target_triples
                .iter()
                .any(|triple| triple == platform)
                && (artifact.kind == "executable-zip"
                    || artifact
                        .assets
                        .iter()
                        .any(|asset| asset.kind == "executable" && !asset.path.is_empty()))
        })
        .with_context(|| i18n.update_err_platform_artifact(platform))
}

fn artifact_executable_path<'a>(artifact: &'a DistArtifact, i18n: &I18n) -> Result<&'a str> {
    let executable = artifact
        .assets
        .iter()
        .find(|asset| asset.kind == "executable" && !asset.path.is_empty())
        .context(i18n.update_err_executable_asset())?;

    Ok(executable.path.as_str())
}

fn download_and_install(
    i18n: &I18n,
    release: &LatestRelease,
    install_path: &PathBuf,
) -> Result<()> {
    let github_proxy = env::var("GITHUB_PROXY").ok();

    let archive_url = format!(
        "https://github.com/{repo}/releases/download/{tag}/{archive}",
        repo = REPO,
        tag = release.tag,
        archive = release.archive_name
    );
    let archive_url = apply_github_proxy(&github_proxy, &archive_url);

    // Create temp directory
    let temp_dir = tempfile::tempdir().context(i18n.update_err_temp_dir())?;
    let temp_archive = temp_dir.path().join(&release.archive_name);
    let extracted_name = Path::new(&release.executable_path)
        .file_name()
        .context(i18n.update_err_executable_asset())?;
    let temp_binary = temp_dir.path().join(extracted_name);

    // Download archive
    download_file(i18n, &archive_url, &temp_archive)?;

    // Verify checksum
    println!("  {}", i18n.update_verifying());
    let actual_hash = compute_sha256(i18n, &temp_archive)?;

    if actual_hash != release.archive_sha256 {
        anyhow::bail!(
            "{}",
            i18n.err_checksum_mismatch(
                &temp_archive.display().to_string(),
                &release.archive_sha256,
                &actual_hash
            )
        );
    }

    extract_binary_from_archive(
        &temp_archive,
        &release.archive_name,
        &release.executable_path,
        &temp_binary,
        i18n,
    )?;

    // Replace binary
    println!("  {}", i18n.update_installing());
    replace_binary(i18n, &temp_binary, install_path)?;

    Ok(())
}

fn extract_binary_from_archive(
    archive_path: &Path,
    archive_name: &str,
    entry_path: &str,
    out_path: &Path,
    i18n: &I18n,
) -> Result<()> {
    if archive_name.ends_with(".tar.xz") {
        return extract_binary_from_tar_xz(archive_path, entry_path, out_path, i18n);
    }
    if archive_name.ends_with(".zip") {
        return extract_binary_from_zip(archive_path, entry_path, out_path, i18n);
    }

    anyhow::bail!("{}", i18n.update_err_extract_archive());
}

fn extract_binary_from_tar_xz(
    archive_path: &Path,
    entry_path: &str,
    out_path: &Path,
    i18n: &I18n,
) -> Result<()> {
    let file = File::open(archive_path).context(i18n.update_err_extract_archive())?;
    let decoder = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .context(i18n.update_err_extract_archive())?
    {
        let mut entry = entry.context(i18n.update_err_extract_archive())?;
        let path = entry.path().context(i18n.update_err_extract_archive())?;
        if path == Path::new(entry_path) {
            entry
                .unpack(out_path)
                .context(i18n.update_err_extract_archive())?;
            return Ok(());
        }
    }

    anyhow::bail!("{}", i18n.update_err_executable_asset())
}

fn extract_binary_from_zip(
    archive_path: &Path,
    entry_path: &str,
    out_path: &Path,
    i18n: &I18n,
) -> Result<()> {
    let file = File::open(archive_path).context(i18n.update_err_extract_archive())?;
    let mut archive = zip::ZipArchive::new(file).context(i18n.update_err_extract_archive())?;
    let mut entry = archive
        .by_name(entry_path)
        .context(i18n.update_err_executable_asset())?;
    let mut out = File::create(out_path).context(i18n.update_err_extract_archive())?;
    io::copy(&mut entry, &mut out).context(i18n.update_err_extract_archive())?;

    Ok(())
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
        let curl_status = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(dest)
            .arg(url)
            .stderr(Stdio::inherit())
            .status();

        match &curl_status {
            Ok(s) if s.success() => {
                return Ok(());
            }
            _ => {}
        }

        // Try wget as fallback
        let wget_status = Command::new("wget")
            .args(["-q", "-O"])
            .arg(dest)
            .arg(url)
            .status();

        return match wget_status {
            Ok(status) if status.success() => Ok(()),
            Ok(_) => Err(anyhow::anyhow!("{}", i18n.err_download_failed())),
            Err(err) => match curl_status {
                Ok(_) => Err(anyhow::anyhow!("{}", i18n.err_download_failed())),
                Err(_) => Err(err).context(i18n.err_download_no_tool()),
            },
        };
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

        return Ok(());
    }

    #[cfg(not(any(unix, windows)))]
    Ok(())
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

struct RulesFile {
    target: InitTarget,
    path: PathBuf,
    rules: String,
}

fn check_rules_after_update(i18n: &I18n, install_path: &PathBuf) -> Result<()> {
    let mut stdin = io::stdin().lock();
    check_rules_after_update_with_reader(i18n, install_path, &mut stdin)
}

fn check_rules_after_update_with_reader(
    i18n: &I18n,
    install_path: &PathBuf,
    reader: &mut dyn BufRead,
) -> Result<()> {
    let rules_files = find_rules_files();
    if rules_files.is_empty() {
        return Ok(());
    }

    println!("{}", i18n.update_rules_checking());

    for file in rules_files {
        let expected_with_pueue = rules_for_target_with_pueue(i18n, file.target, true);
        let expected_without_pueue = rules_for_target_with_pueue(i18n, file.target, false);

        let unmodified = file.rules == expected_with_pueue || file.rules == expected_without_pueue;
        if unmodified {
            println!(
                "{}",
                i18n.update_rules_outdated(&file.path.display().to_string())
            );
            if prompt_yes_no_with_reader(i18n.update_rules_confirm_update(), reader)? {
                run_init_with_binary(i18n, install_path, file.target)?;
            } else {
                println!("{}", i18n.update_rules_skipped());
            }
            println!();
            continue;
        }

        let reference =
            pick_reference_template(&file.rules, &expected_with_pueue, &expected_without_pueue);

        println!(
            "{}",
            i18n.update_rules_modified(&file.path.display().to_string())
        );
        print_rules_diff(
            i18n,
            &file.path.display().to_string(),
            reference,
            &file.rules,
        );
        if prompt_yes_no_with_reader(i18n.update_rules_confirm_overwrite(), reader)? {
            run_init_with_binary(i18n, install_path, file.target)?;
        } else {
            println!("{}", i18n.update_rules_skipped());
        }
        println!();
    }

    Ok(())
}

fn find_rules_files() -> Vec<RulesFile> {
    let mut files = Vec::new();
    let Ok(home) = home_dir() else {
        return files;
    };

    push_rules_file(
        &mut files,
        home.join(".claude").join("rules").join("shnote.md"),
        InitTarget::Claude,
    );
    push_rules_file(
        &mut files,
        home.join(".claude").join("CLAUDE.md"),
        InitTarget::Claude,
    );
    push_rules_file(
        &mut files,
        home.join(".codex").join("AGENTS.md"),
        InitTarget::Codex,
    );
    push_rules_file(
        &mut files,
        home.join(".gemini").join("GEMINI.md"),
        InitTarget::Gemini,
    );

    files
}

fn push_rules_file(files: &mut Vec<RulesFile>, path: PathBuf, target: InitTarget) {
    if !path.exists() {
        return;
    }

    let Ok(content) = fs::read_to_string(&path) else {
        return;
    };

    let Some(rules) = extract_shnote_rules(&content) else {
        return;
    };

    files.push(RulesFile {
        target,
        path,
        rules,
    });
}

fn extract_shnote_rules(content: &str) -> Option<String> {
    let start_idx = content.find(SHNOTE_MARKER_START)?;
    let rules_start = start_idx + SHNOTE_MARKER_START.len();
    let rules_end = content[rules_start..]
        .find(SHNOTE_MARKER_END)
        .map(|i| rules_start + i)
        .unwrap_or(content.len());

    Some(content[rules_start..rules_end].to_string())
}

fn pick_reference_template<'a>(rules: &str, a: &'a str, b: &'a str) -> &'a str {
    let score_a = diff_score(rules, a);
    let score_b = diff_score(rules, b);
    if score_a <= score_b {
        a
    } else {
        b
    }
}

fn print_rules_diff(i18n: &I18n, path: &str, expected: &str, actual: &str) {
    println!("{}", i18n.update_rules_diff_header(path));
    println!("--- {}", i18n.update_rules_diff_base());
    println!("+++ {}", i18n.update_rules_diff_current());
    print!("{}", render_diff(expected, actual));
}

fn render_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let dp = lcs_table(&old_lines, &new_lines);

    let mut out = String::new();
    let mut i = 0;
    let mut j = 0;
    while i < old_lines.len() && j < new_lines.len() {
        if old_lines[i] == new_lines[j] {
            out.push(' ');
            out.push_str(old_lines[i]);
            out.push('\n');
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            out.push('-');
            out.push_str(old_lines[i]);
            out.push('\n');
            i += 1;
        } else {
            out.push('+');
            out.push_str(new_lines[j]);
            out.push('\n');
            j += 1;
        }
    }
    while i < old_lines.len() {
        out.push('-');
        out.push_str(old_lines[i]);
        out.push('\n');
        i += 1;
    }
    while j < new_lines.len() {
        out.push('+');
        out.push_str(new_lines[j]);
        out.push('\n');
        j += 1;
    }

    out
}

fn diff_score(old: &str, new: &str) -> usize {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let dp = lcs_table(&old_lines, &new_lines);

    let mut score = 0;
    let mut i = 0;
    let mut j = 0;
    while i < old_lines.len() && j < new_lines.len() {
        if old_lines[i] == new_lines[j] {
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            score += 1;
            i += 1;
        } else {
            score += 1;
            j += 1;
        }
    }
    score + (old_lines.len() - i) + (new_lines.len() - j)
}

fn lcs_table(old_lines: &[&str], new_lines: &[&str]) -> Vec<Vec<usize>> {
    let mut dp = vec![vec![0; new_lines.len() + 1]; old_lines.len() + 1];
    for i in (0..old_lines.len()).rev() {
        for j in (0..new_lines.len()).rev() {
            if old_lines[i] == new_lines[j] {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }
    dp
}

fn prompt_yes_no_with_reader(prompt: &str, reader: &mut dyn BufRead) -> Result<bool> {
    print!("{prompt} [y/N] ");
    io::stdout().flush()?;
    let mut input = String::new();
    reader.read_line(&mut input)?;
    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

fn run_init_with_binary(i18n: &I18n, install_path: &PathBuf, target: InitTarget) -> Result<()> {
    let status = Command::new(install_path)
        .arg("--lang")
        .arg(i18n.lang_tag())
        .arg("init")
        .arg(init_target_arg(target))
        .status()
        .context(i18n.update_rules_err_init())?;

    if !status.success() {
        anyhow::bail!("{}", i18n.update_rules_err_init());
    }

    Ok(())
}

fn init_target_arg(target: InitTarget) -> &'static str {
    match target {
        InitTarget::Claude => "claude",
        InitTarget::Codex => "codex",
        InitTarget::Gemini => "gemini",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::init::{rules_for_target_with_pueue, SHNOTE_MARKER_END, SHNOTE_MARKER_START};
    #[cfg(unix)]
    use crate::test_support::write_executable;
    use crate::test_support::{env_lock, EnvVarGuard};
    use std::io::Cursor;
    use tempfile::TempDir;

    const DIST_MANIFEST_FIXTURE: &str = r#"{
        "announcement_tag": "v0.3.1",
        "artifacts": [
            {
                "name": "sha256.sum",
                "kind": "unified-checksum"
            },
            {
                "name": "shnote-x86_64-apple-darwin.tar.xz",
                "target_triples": ["x86_64-apple-darwin"],
                "checksums": { "sha256": "deadbeef" },
                "assets": [
                    { "kind": "executable", "path": "shnote" }
                ]
            },
            {
                "name": "shnote-x86_64-pc-windows-msvc.zip",
                "target_triples": ["x86_64-pc-windows-msvc"],
                "checksums": { "sha256": "feedface" },
                "assets": [
                    { "kind": "executable", "path": "shnote.exe" }
                ]
            }
        ]
    }"#;

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
    fn parse_dist_manifest_reads_latest_tag() {
        let i18n = I18n::new(Lang::En);
        let manifest = parse_dist_manifest(DIST_MANIFEST_FIXTURE, &i18n).unwrap();
        assert_eq!(manifest.announcement_tag, "v0.3.1");
    }

    #[test]
    fn select_platform_artifact_picks_matching_target() {
        let i18n = I18n::new(Lang::En);
        let manifest = parse_dist_manifest(DIST_MANIFEST_FIXTURE, &i18n).unwrap();
        let artifact = select_platform_artifact(&manifest, "x86_64-apple-darwin", &i18n).unwrap();
        assert_eq!(artifact.name, "shnote-x86_64-apple-darwin.tar.xz");
    }

    #[test]
    fn latest_release_from_manifest_reads_tag_and_archive() {
        let i18n = I18n::new(Lang::En);
        let release =
            latest_release_from_manifest(DIST_MANIFEST_FIXTURE, "x86_64-apple-darwin", &i18n)
                .unwrap();
        assert_eq!(release.version, "0.3.1");
        assert_eq!(release.tag, "v0.3.1");
        assert_eq!(release.archive_name, "shnote-x86_64-apple-darwin.tar.xz");
        assert_eq!(release.archive_sha256, "deadbeef");
        assert_eq!(release.executable_path, "shnote");
    }

    #[test]
    fn latest_release_from_manifest_reports_missing_platform() {
        let i18n = I18n::new(Lang::En);
        let err =
            latest_release_from_manifest(DIST_MANIFEST_FIXTURE, "thumbv7em-none-eabihf", &i18n)
                .unwrap_err();
        assert!(err.to_string().contains("thumbv7em-none-eabihf"));
    }

    #[test]
    fn latest_release_from_manifest_reports_missing_executable_asset() {
        let i18n = I18n::new(Lang::En);
        let err = latest_release_from_manifest(
            r#"{
                "announcement_tag": "v0.3.1",
                "artifacts": [
                    {
                        "name": "shnote-x86_64-apple-darwin.tar.xz",
                        "kind": "executable-zip",
                        "target_triples": ["x86_64-apple-darwin"],
                        "checksums": { "sha256": "deadbeef" },
                        "assets": []
                    }
                ]
            }"#,
            "x86_64-apple-darwin",
            &i18n,
        )
        .unwrap_err();
        assert!(err.to_string().contains("executable"));
    }

    #[test]
    fn extract_shnote_rules_uses_markers() {
        let content = format!(
            "before{start}RULES{end}after",
            start = SHNOTE_MARKER_START,
            end = SHNOTE_MARKER_END
        );
        assert_eq!(extract_shnote_rules(&content), Some("RULES".to_string()));
    }

    #[test]
    fn extract_shnote_rules_handles_missing_end_marker() {
        let content = format!("before{start}RULES", start = SHNOTE_MARKER_START);
        assert_eq!(extract_shnote_rules(&content), Some("RULES".to_string()));
    }

    #[test]
    fn render_diff_marks_changes() {
        let diff = render_diff("a\nb\n", "a\nc\n");
        assert!(diff.contains("-b"));
        assert!(diff.contains("+c"));
    }

    #[test]
    fn pick_reference_template_prefers_closer_match() {
        let a = "line1\nline2\n";
        let b = "line1\nline3\n";
        let rules = "line1\nline2\nline4\n";
        let chosen = pick_reference_template(rules, a, b);
        assert_eq!(chosen, a);
    }

    #[test]
    fn prompt_yes_no_with_reader_accepts_yes() {
        let mut input = Cursor::new("y\n");
        assert!(prompt_yes_no_with_reader("ok?", &mut input).unwrap());
    }

    #[test]
    fn prompt_yes_no_with_reader_rejects_default() {
        let mut input = Cursor::new("\n");
        assert!(!prompt_yes_no_with_reader("ok?", &mut input).unwrap());
    }

    #[test]
    fn extract_binary_from_tar_xz_uses_manifest_asset_path() {
        let i18n = I18n::new(Lang::En);
        let temp_dir = TempDir::new().unwrap();
        let archive = write_tar_xz_fixture(&temp_dir, "shnote", b"unix-binary");
        let out = temp_dir.path().join("shnote-out");

        extract_binary_from_archive(
            &archive,
            "shnote-aarch64-apple-darwin.tar.xz",
            "shnote",
            &out,
            &i18n,
        )
        .unwrap();

        assert_eq!(fs::read(&out).unwrap(), b"unix-binary");
    }

    #[test]
    fn extract_binary_from_zip_uses_manifest_asset_path() {
        let i18n = I18n::new(Lang::En);
        let temp_dir = TempDir::new().unwrap();
        let archive = write_zip_fixture(&temp_dir, "shnote.exe", b"windows-binary");
        let out = temp_dir.path().join("shnote.exe");

        extract_binary_from_archive(
            &archive,
            "shnote-x86_64-pc-windows-msvc.zip",
            "shnote.exe",
            &out,
            &i18n,
        )
        .unwrap();

        assert_eq!(fs::read(&out).unwrap(), b"windows-binary");
    }

    #[cfg(unix)]
    #[test]
    fn download_file_prefers_curl() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let tools_dir = temp_dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();

        let curl = tools_dir.join("curl");
        write_executable(
            &curl,
            "#!/bin/sh\n\
            dest=\"\"\n\
            while [ \"$1\" != \"\" ]; do\n\
              if [ \"$1\" = \"-o\" ]; then\n\
                shift\n\
                dest=\"$1\"\n\
              fi\n\
              shift\n\
            done\n\
            echo \"curl\" > \"$dest\"\n\
            exit 0\n",
        )
        .unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let out = temp_dir.path().join("out.txt");
        download_file(&i18n, "https://example.invalid/file", &out).unwrap();
        assert_eq!(fs::read_to_string(&out).unwrap().trim(), "curl");
    }

    #[cfg(unix)]
    #[test]
    fn download_file_falls_back_to_wget() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let tools_dir = temp_dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();

        let curl = tools_dir.join("curl");
        write_executable(&curl, "#!/bin/sh\nexit 1\n").unwrap();

        let wget = tools_dir.join("wget");
        write_executable(
            &wget,
            "#!/bin/sh\n\
            dest=\"\"\n\
            while [ \"$1\" != \"\" ]; do\n\
              if [ \"$1\" = \"-O\" ]; then\n\
                shift\n\
                dest=\"$1\"\n\
              fi\n\
              shift\n\
            done\n\
            echo \"wget\" > \"$dest\"\n\
            exit 0\n",
        )
        .unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let out = temp_dir.path().join("out.txt");
        download_file(&i18n, "https://example.invalid/file", &out).unwrap();
        assert_eq!(fs::read_to_string(&out).unwrap().trim(), "wget");
    }

    #[cfg(unix)]
    #[test]
    fn download_file_errors_when_no_tool_available() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let tools_dir = temp_dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let out = temp_dir.path().join("out.txt");
        let err = download_file(&i18n, "https://example.invalid/file", &out).unwrap_err();
        assert!(err.to_string().contains(i18n.err_download_no_tool()));
    }

    #[cfg(unix)]
    #[test]
    fn download_file_keeps_primary_error_when_wget_missing() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let tools_dir = temp_dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();

        let curl = tools_dir.join("curl");
        write_executable(
            &curl,
            "#!/bin/sh\n\
            echo 'curl: (56) The requested URL returned error: 404' >&2\n\
            exit 56\n",
        )
        .unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let out = temp_dir.path().join("out.txt");
        let err = download_file(&i18n, "https://example.invalid/file", &out).unwrap_err();
        assert!(err.to_string().contains(i18n.err_download_failed()));
        assert!(!err.to_string().contains(i18n.err_download_no_tool()));
    }

    #[cfg(unix)]
    #[test]
    fn compute_sha256_uses_shasum_output() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let tools_dir = temp_dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();

        let shasum = tools_dir.join("shasum");
        write_executable(&shasum, "#!/bin/sh\necho \"deadbeef  $2\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let file = temp_dir.path().join("bin");
        fs::write(&file, "data").unwrap();
        let hash = compute_sha256(&i18n, &file).unwrap();
        assert_eq!(hash, "deadbeef");
    }

    #[cfg(unix)]
    #[test]
    fn fetch_latest_release_downloads_manifest_and_selects_platform_artifact() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let tools_dir = temp_dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();
        let manifest = format!(
            r#"{{
                "announcement_tag": "v0.3.1",
                "artifacts": [
                    {{
                        "name": "shnote-{platform}.tar.xz",
                        "target_triples": ["{platform}"],
                        "checksums": {{ "sha256": "deadbeef" }},
                        "assets": [
                            {{ "kind": "executable", "path": "shnote" }}
                        ]
                    }}
                ]
            }}"#,
            platform = PLATFORM
        );

        let curl = tools_dir.join("curl");
        write_executable(
            &curl,
            &format!(
                "#!/bin/sh\n\
                dest=\"\"\n\
                while [ \"$1\" != \"\" ]; do\n\
                  if [ \"$1\" = \"-o\" ]; then\n\
                    shift\n\
                    dest=\"$1\"\n\
                  fi\n\
                  shift\n\
                done\n\
                /bin/cat <<'EOF' > \"$dest\"\n\
                {}\n\
                EOF\n\
                exit 0\n",
                manifest
            ),
        )
        .unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let release = fetch_latest_release(&i18n).unwrap();
        assert_eq!(release.version, "0.3.1");
        assert_eq!(release.archive_name, format!("shnote-{PLATFORM}.tar.xz"));
        assert_eq!(release.executable_path, "shnote");
    }

    #[cfg(unix)]
    #[test]
    fn download_and_install_writes_extracted_binary() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let tools_dir = temp_dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();

        let archive = write_tar_xz_fixture(&temp_dir, "shnote", b"binary");
        let curl = tools_dir.join("curl");
        write_executable(
            &curl,
            &format!(
                "#!/bin/sh\n\
                dest=\"\"\n\
                while [ \"$1\" != \"\" ]; do\n\
                  if [ \"$1\" = \"-o\" ]; then\n\
                    shift\n\
                    dest=\"$1\"\n\
                  fi\n\
                  shift\n\
                done\n\
                /bin/cp \"{}\" \"$dest\"\n\
                exit 0\n",
                archive.display()
            ),
        )
        .unwrap();

        let shasum = tools_dir.join("shasum");
        write_executable(&shasum, "#!/bin/sh\necho \"archivehash  $2\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let install_dir = TempDir::new().unwrap();
        let install_path = install_dir.path().join("shnote");
        let release = LatestRelease {
            version: "0.3.1".to_string(),
            tag: "v0.3.1".to_string(),
            archive_name: "shnote-x86_64-apple-darwin.tar.xz".to_string(),
            archive_sha256: "archivehash".to_string(),
            executable_path: "shnote".to_string(),
        };

        download_and_install(&i18n, &release, &install_path).unwrap();

        assert_eq!(fs::read(&install_path).unwrap(), b"binary");
    }

    #[cfg(unix)]
    #[test]
    fn download_and_install_rejects_bad_checksum() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let tools_dir = temp_dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();

        let archive = write_tar_xz_fixture(&temp_dir, "shnote", b"binary");
        let curl = tools_dir.join("curl");
        write_executable(
            &curl,
            &format!(
                "#!/bin/sh\n\
                dest=\"\"\n\
                while [ \"$1\" != \"\" ]; do\n\
                  if [ \"$1\" = \"-o\" ]; then\n\
                    shift\n\
                    dest=\"$1\"\n\
                  fi\n\
                  shift\n\
                done\n\
                /bin/cp \"{}\" \"$dest\"\n\
                exit 0\n",
                archive.display()
            ),
        )
        .unwrap();

        let shasum = tools_dir.join("shasum");
        write_executable(&shasum, "#!/bin/sh\necho \"bad  $2\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let install_dir = TempDir::new().unwrap();
        let install_path = install_dir.path().join("shnote");
        let release = LatestRelease {
            version: "0.3.1".to_string(),
            tag: "v0.3.1".to_string(),
            archive_name: "shnote-x86_64-apple-darwin.tar.xz".to_string(),
            archive_sha256: "archivehash".to_string(),
            executable_path: "shnote".to_string(),
        };

        let err = download_and_install(&i18n, &release, &install_path).unwrap_err();
        assert!(err.to_string().contains("checksum"));
    }

    #[cfg(unix)]
    #[test]
    fn check_rules_after_update_updates_unmodified_rules() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let rules = rules_for_target_with_pueue(&i18n, InitTarget::Codex, true);
        let codex_dir = temp_dir.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let rules_path = codex_dir.join("AGENTS.md");
        let content = format!(
            "prefix{start}{rules}{end}suffix",
            start = SHNOTE_MARKER_START,
            end = SHNOTE_MARKER_END,
            rules = rules
        );
        fs::write(&rules_path, content).unwrap();

        let install_dir = TempDir::new().unwrap();
        let output_path = install_dir.path().join("args.txt");
        let binary_path = install_dir.path().join("shnote");
        write_executable(
            &binary_path,
            &format!(
                "#!/bin/sh\n\
                echo \"$@\" > \"{}\"\n\
                exit 0\n",
                output_path.display()
            ),
        )
        .unwrap();

        let mut input = Cursor::new("y\n");
        check_rules_after_update_with_reader(&i18n, &binary_path, &mut input).unwrap();

        let args = fs::read_to_string(&output_path).unwrap();
        assert!(args.contains("--lang"));
        assert!(args.contains("init"));
        assert!(args.contains("codex"));
    }

    #[cfg(unix)]
    #[test]
    fn check_rules_after_update_reports_modified_rules() {
        let _lock = env_lock();
        let i18n = I18n::new(Lang::En);

        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let codex_dir = temp_dir.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let rules_path = codex_dir.join("AGENTS.md");
        let content = format!(
            "prefix{start}custom rules{end}suffix",
            start = SHNOTE_MARKER_START,
            end = SHNOTE_MARKER_END
        );
        fs::write(&rules_path, content).unwrap();

        let install_dir = TempDir::new().unwrap();
        let binary_path = install_dir.path().join("shnote");
        write_executable(&binary_path, "#!/bin/sh\nexit 0\n").unwrap();

        let mut input = Cursor::new("n\n");
        check_rules_after_update_with_reader(&i18n, &binary_path, &mut input).unwrap();
    }

    fn write_tar_xz_fixture(temp_dir: &TempDir, entry_path: &str, contents: &[u8]) -> PathBuf {
        let archive_path = temp_dir.path().join("fixture.tar.xz");
        let file = File::create(&archive_path).unwrap();
        let encoder = xz2::write::XzEncoder::new(file, 6);
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_path(entry_path).unwrap();
        header.set_size(contents.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append(&header, contents).unwrap();
        let encoder = builder.into_inner().unwrap();
        encoder.finish().unwrap();
        archive_path
    }

    fn write_zip_fixture(temp_dir: &TempDir, entry_path: &str, contents: &[u8]) -> PathBuf {
        let archive_path = temp_dir.path().join("fixture.zip");
        let file = File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        writer.start_file(entry_path, options).unwrap();
        writer.write_all(contents).unwrap();
        writer.finish().unwrap();
        archive_path
    }
}
