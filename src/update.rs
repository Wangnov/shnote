use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::cli::{InitTarget, UpdateArgs};
use crate::config::home_dir;
use crate::i18n::I18n;
use crate::info::{get_install_path, PLATFORM, REPO, VERSION};
use crate::init::{rules_for_target_with_pueue, SHNOTE_MARKER_END, SHNOTE_MARKER_START};

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
    println!();

    check_rules_after_update(i18n, &install_path)?;

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

        let curl = tools_dir.join("curl");
        write_executable(&curl, "#!/bin/sh\nexit 1\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let out = temp_dir.path().join("out.txt");
        let err = download_file(&i18n, "https://example.invalid/file", &out).unwrap_err();
        assert!(err.to_string().contains(i18n.err_download_no_tool()));
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
    fn download_and_install_writes_binary() {
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
            case \"$dest\" in\n\
              *.sha256) printf \"deadbeef\" > \"$dest\" ;;\n\
              *) printf \"binary\" > \"$dest\" ;;\n\
            esac\n\
            exit 0\n",
        )
        .unwrap();

        let shasum = tools_dir.join("shasum");
        write_executable(&shasum, "#!/bin/sh\necho \"deadbeef  $2\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let install_dir = TempDir::new().unwrap();
        let install_path = install_dir.path().join("shnote");
        download_and_install(&i18n, "0.0.0", &install_path).unwrap();

        assert_eq!(fs::read_to_string(&install_path).unwrap(), "binary");
    }

    #[cfg(unix)]
    #[test]
    fn download_and_install_rejects_bad_checksum() {
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
            case \"$dest\" in\n\
              *.sha256) printf \"deadbeef\" > \"$dest\" ;;\n\
              *) printf \"binary\" > \"$dest\" ;;\n\
            esac\n\
            exit 0\n",
        )
        .unwrap();

        let shasum = tools_dir.join("shasum");
        write_executable(&shasum, "#!/bin/sh\necho \"bad  $2\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", &tools_dir);

        let install_dir = TempDir::new().unwrap();
        let install_path = install_dir.path().join("shnote");
        let err = download_and_install(&i18n, "0.0.0", &install_path).unwrap_err();
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
}
