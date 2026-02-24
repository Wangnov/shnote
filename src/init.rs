use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use which::which;

use crate::cli::{InitTarget, Scope};
use crate::config::home_dir;
use crate::i18n::{I18n, Lang};

/// Embedded shnote rules content (shared by all targets)
const SHNOTE_RULES_BASE: &str = r#"# 运行 shell 命令必须使用 shnote 包装

shnote 用于在执行 shell 命令前补充 WHAT/WHY，帮助用户理解命令意图。

## 核心约束

1. 执行类子命令（`run` / `py` / `node` / `pip` / `npm` / `npx`）使用：
   `shnote --what "<做什么>" --why "<为什么>" <subcommand> ...`
2. WHAT 写动作，WHY 写原因，保持简短、具体。
3. 默认 `header_stream=auto`：`stdout` 作为数据通道，`stderr` 作为说明通道。
4. 若命令包含管道、重定向、逻辑连接或分号（如 `|`、`|&`、`>`、`>>`、`<`、`2>&1`、`&&`、`||`、`;`），请把整段命令放在同一对引号中，作为 shnote 的一个参数传入（可用双引号；需要避免变量展开时可用单引号）。
5. 以下场景可能受 WHAT/WHY 影响，需要额外处理 `stderr`：
   - 合并双流（如 `2>&1`、`|&`）
   - 把 `stderr` 非空当作失败信号
   - 直接解析整段终端日志（未区分 stdout/stderr）

## 不需要通过 shnote 的操作

{{NON_SHNOTE_TOOLS}}
"#;

const SHNOTE_RULES_BASE_EN: &str = r#"# Wrap shell commands with shnote

shnote adds WHAT/WHY before shell execution so users can quickly understand intent.

## Core Constraints

1. For execution subcommands (`run` / `py` / `node` / `pip` / `npm` / `npx`), use:
   `shnote --what "<what>" --why "<why>" <subcommand> ...`
2. Keep WHAT action-focused and WHY context-focused; both should be concise.
3. Default `header_stream=auto`: treat `stdout` as data channel and `stderr` as annotation channel.
4. If a command uses pipes/redirection/operators (for example `|`, `|&`, `>`, `>>`, `<`, `2>&1`, `&&`, `||`, `;`), pass the whole command as one quoted string argument to shnote (double quotes are preferred; use single quotes when you need to suppress variable expansion).
5. WHAT/WHY may still affect downstream handling in these cases; split/filter `stderr` as needed:
   - Merging both streams (`2>&1`, `|&`)
   - Systems treating any non-empty `stderr` as failure
   - Parsers that consume raw terminal logs without stream separation

## Operations that do not need shnote

{{NON_SHNOTE_TOOLS}}
"#;

/// Codex-specific additions to the shnote rules (ZH)
const SHNOTE_RULES_CODEX_EXTRA: &str = r#"## Codex 额外规则

1. 只读查看文件内容时，直接用 shell（如 `cat` / `head` / `tail` / `sed` / `nl` / `rg`），不使用 shnote。
2. 编辑文件优先使用 `apply_patch`（或内建编辑工具）。
"#;

/// Codex-specific additions to the shnote rules (EN)
const SHNOTE_RULES_CODEX_EXTRA_EN: &str = r#"## Codex Extra Rules

1. For read-only file display, use direct shell (`cat` / `head` / `tail` / `sed` / `nl` / `rg`) without shnote.
2. For edits, prefer `apply_patch` (or built-in edit tools).
"#;

/// Claude-specific additions to the shnote rules (ZH)
const SHNOTE_RULES_CLAUDE_EXTRA: &str = r#"## Claude Code 额外规则

**只有 Bash 工具才必须使用 shnote**；Read / Write / Edit 等工具一律不使用 shnote。
"#;

/// Claude-specific additions to the shnote rules (EN)
const SHNOTE_RULES_CLAUDE_EXTRA_EN: &str = r#"## Claude Code Extra Rules

**Only the Bash tool must use shnote**; Read / Write / Edit tools must not use shnote.
"#;

/// Gemini-specific additions to the shnote rules (ZH)
const SHNOTE_RULES_GEMINI_EXTRA: &str = r#"## Gemini 额外规则

**仅 run_shell_command 需要使用 shnote**；list_directory / read_file / write_file / replace 等工具一律不使用 shnote。
"#;

/// Gemini-specific additions to the shnote rules (EN)
const SHNOTE_RULES_GEMINI_EXTRA_EN: &str = r#"## Gemini Extra Rules

**Only run_shell_command uses shnote**; list_directory / read_file / write_file / replace tools must not use shnote.
"#;

/// Marker to identify shnote rules section in append mode
pub(crate) const SHNOTE_MARKER_START: &str = "\n<!-- shnote rules start -->\n";
pub(crate) const SHNOTE_MARKER_END: &str = "\n<!-- shnote rules end -->\n";

fn non_shnote_tools_for_target(lang: Lang, target: InitTarget) -> &'static str {
    match (lang, target) {
        (Lang::Zh, InitTarget::Codex) => "1. **只读查看文件**：直接用 shell，不通过 shnote。\n2. **非 shell 的内建工具**（读文件、列目录、编辑文件等）不通过 shnote。",
        (Lang::En, InitTarget::Codex) => "1. **Read-only file viewing**: use direct shell, not shnote.\n2. **Non-shell built-in tools** (read/list/edit operations) do not need shnote.",
        (Lang::Zh, InitTarget::Claude) => "1. **仅 Bash 工具必须使用 shnote**：Read / Write / Edit 等工具不使用 shnote。",
        (Lang::En, InitTarget::Claude) => "1. **Only the Bash tool must use shnote**: Read / Write / Edit tools do not use shnote.",
        (Lang::Zh, InitTarget::Gemini) => "1. **仅 run_shell_command 需要使用 shnote**：list_directory / read_file / write_file / replace 等工具不使用 shnote。",
        (Lang::En, InitTarget::Gemini) => "1. **Only run_shell_command needs shnote**: list_directory / read_file / write_file / replace do not use shnote.",
    }
}

fn extra_rules_for_target(lang: Lang, target: InitTarget) -> Option<&'static str> {
    match (lang, target) {
        (Lang::Zh, InitTarget::Codex) => Some(SHNOTE_RULES_CODEX_EXTRA),
        (Lang::En, InitTarget::Codex) => Some(SHNOTE_RULES_CODEX_EXTRA_EN),
        (Lang::Zh, InitTarget::Claude) => Some(SHNOTE_RULES_CLAUDE_EXTRA),
        (Lang::En, InitTarget::Claude) => Some(SHNOTE_RULES_CLAUDE_EXTRA_EN),
        (Lang::Zh, InitTarget::Gemini) => Some(SHNOTE_RULES_GEMINI_EXTRA),
        (Lang::En, InitTarget::Gemini) => Some(SHNOTE_RULES_GEMINI_EXTRA_EN),
    }
}

pub(crate) fn rules_for_target_with_pueue(
    i18n: &I18n,
    target: InitTarget,
    _include_pueue: bool,
) -> String {
    let template = match i18n.lang() {
        Lang::Zh => SHNOTE_RULES_BASE,
        Lang::En => SHNOTE_RULES_BASE_EN,
    };
    let mut rules = template.replace(
        "{{NON_SHNOTE_TOOLS}}",
        non_shnote_tools_for_target(i18n.lang(), target),
    );
    if let Some(extra) = extra_rules_for_target(i18n.lang(), target) {
        rules.push_str("\n\n");
        rules.push_str(extra);
    }
    rules
}

fn rules_for_target(i18n: &I18n, target: InitTarget) -> String {
    rules_for_target_with_pueue(i18n, target, false)
}

pub fn run_init(i18n: &I18n, target: InitTarget, scope: Scope) -> Result<()> {
    match target {
        InitTarget::Claude => init_claude(i18n, scope),
        InitTarget::Codex => init_codex(i18n, scope),
        InitTarget::Gemini => init_gemini(i18n, scope),
    }
}

/// Get base directory for the given scope
fn get_base_dir(i18n: &I18n, scope: Scope) -> Result<PathBuf> {
    match scope {
        Scope::User => home_dir().context(i18n.err_home_dir()),
        Scope::Project => std::env::current_dir().context(i18n.err_current_dir()),
    }
}

fn init_claude(i18n: &I18n, scope: Scope) -> Result<()> {
    let probe = probe_cli_tool(i18n, "claude");
    let base = get_base_dir(i18n, scope)?;
    let rules = rules_for_target(i18n, InitTarget::Claude);

    // Claude Code >= 2.0.64 supports ~/.claude/rules/*.md.
    // For older versions (or when version cannot be determined), append rules to ~/.claude/CLAUDE.md.
    let claude_supports_rules = probe
        .version
        .as_deref()
        .and_then(parse_semver_from_text)
        .is_some_and(|v| v >= SemVer::new(2, 0, 64));

    let old_claude_md = base.join(".claude").join("CLAUDE.md");

    if claude_supports_rules {
        let rules_dir = base.join(".claude").join("rules");
        fs::create_dir_all(&rules_dir)
            .context(i18n.err_create_dir(&rules_dir.display().to_string()))?;
        let target_file = rules_dir.join("shnote.md");

        // Check if old CLAUDE.md has shnote rules that need migration
        let migrated = if old_claude_md.exists() {
            migrate_shnote_rules(i18n, &old_claude_md, &target_file, &rules)?
        } else {
            false
        };

        if !migrated {
            // No migration needed, just write the rules file
            fs::write(&target_file, &rules)
                .context(i18n.err_write_file(&target_file.display().to_string()))?;
        }

        println!(
            "{}",
            i18n.init_claude_success(&target_file.display().to_string())
        );
        if migrated {
            println!(
                "{}",
                i18n.init_migrated_from(&old_claude_md.display().to_string())
            );
            println!(
                "{}",
                i18n.init_old_rules_cleaned(&old_claude_md.display().to_string())
            );
        }
    } else {
        let claude_dir = base.join(".claude");
        fs::create_dir_all(&claude_dir)
            .context(i18n.err_create_dir(&claude_dir.display().to_string()))?;
        let target_file = claude_dir.join("CLAUDE.md");
        append_rules(i18n, &target_file, &rules)?;
        println!(
            "{}",
            i18n.init_claude_success(&target_file.display().to_string())
        );
    }

    Ok(())
}

/// Migrate shnote rules from old CLAUDE.md to new rules file.
/// Returns true if migration was performed, false if no old rules found.
fn migrate_shnote_rules(
    i18n: &I18n,
    old_file: &Path,
    new_file: &Path,
    rules: &str,
) -> Result<bool> {
    let content = fs::read_to_string(old_file)
        .context(i18n.err_read_file(&old_file.display().to_string()))?;

    // Check if shnote rules exist in old file
    let Some(start_idx) = content.find(SHNOTE_MARKER_START) else {
        return Ok(false);
    };

    // Extract the shnote rules content (between markers)
    let rules_start = start_idx + SHNOTE_MARKER_START.len();
    let rules_end = content[rules_start..]
        .find(SHNOTE_MARKER_END)
        .map(|i| rules_start + i)
        .unwrap_or(content.len());

    let old_rules = content[rules_start..rules_end].to_string();

    // Write extracted rules to new file (use latest rules, not old content)
    // This ensures we always have the latest version
    fs::write(new_file, rules).context(i18n.err_write_file(&new_file.display().to_string()))?;

    // Remove shnote rules from old file
    let marker_end_idx = content
        .find(SHNOTE_MARKER_END)
        .map(|i| i + SHNOTE_MARKER_END.len())
        .unwrap_or(content.len());

    let mut new_content = String::new();
    new_content.push_str(&content[..start_idx]);
    new_content.push_str(&content[marker_end_idx..]);

    // Trim trailing newlines that might have been left behind
    let new_content = new_content.trim_end().to_string();

    if new_content.is_empty() {
        // If the file would be empty, just delete it
        fs::remove_file(old_file).context(i18n.err_write_file(&old_file.display().to_string()))?;
    } else {
        fs::write(old_file, new_content)
            .context(i18n.err_write_file(&old_file.display().to_string()))?;
    }

    // Suppress unused variable warning - we extract it for potential future use
    let _ = old_rules;

    Ok(true)
}

fn init_codex(i18n: &I18n, scope: Scope) -> Result<()> {
    let _ = probe_cli_tool(i18n, "codex");
    let base = get_base_dir(i18n, scope)?;
    let rules = rules_for_target(i18n, InitTarget::Codex);
    let codex_dir = base.join(".codex");
    let target_file = codex_dir.join("AGENTS.md");

    // Create directory if needed
    fs::create_dir_all(&codex_dir)
        .context(i18n.err_create_dir(&codex_dir.display().to_string()))?;

    append_rules(i18n, &target_file, &rules)?;

    println!(
        "{}",
        i18n.init_codex_success(&target_file.display().to_string())
    );
    Ok(())
}

fn init_gemini(i18n: &I18n, scope: Scope) -> Result<()> {
    let _ = probe_cli_tool(i18n, "gemini");
    let base = get_base_dir(i18n, scope)?;
    let rules = rules_for_target(i18n, InitTarget::Gemini);
    let gemini_dir = base.join(".gemini");
    let target_file = gemini_dir.join("GEMINI.md");

    // Create directory if needed
    fs::create_dir_all(&gemini_dir)
        .context(i18n.err_create_dir(&gemini_dir.display().to_string()))?;

    append_rules(i18n, &target_file, &rules)?;

    println!(
        "{}",
        i18n.init_gemini_success(&target_file.display().to_string())
    );
    Ok(())
}

fn append_rules(i18n: &I18n, target_file: &PathBuf, rules: &str) -> Result<()> {
    let mut content = if target_file.exists() {
        fs::read_to_string(target_file)
            .context(i18n.err_read_file(&target_file.display().to_string()))?
    } else {
        String::new()
    };

    // Check if shnote rules already exist
    if content.contains(SHNOTE_MARKER_START) {
        // Replace existing rules
        let start_idx = content.find(SHNOTE_MARKER_START).unwrap();
        let end_idx = content
            .find(SHNOTE_MARKER_END)
            .map(|i| i + SHNOTE_MARKER_END.len())
            .unwrap_or(content.len());

        let mut new_content = String::new();
        new_content.push_str(&content[..start_idx]);
        new_content.push_str(SHNOTE_MARKER_START);
        new_content.push_str(rules);
        new_content.push_str(SHNOTE_MARKER_END);
        new_content.push_str(&content[end_idx..]);

        fs::write(target_file, new_content)
            .context(i18n.err_write_file(&target_file.display().to_string()))?;

        println!("{}", i18n.init_rules_updated());
    } else {
        // Append new rules (rewrite the file to keep behavior deterministic and testable)
        content.push_str(SHNOTE_MARKER_START);
        content.push_str(rules);
        content.push_str(SHNOTE_MARKER_END);

        fs::write(target_file, content)
            .context(i18n.err_write_file(&target_file.display().to_string()))?;

        println!("{}", i18n.init_rules_appended());
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct ToolProbe {
    #[allow(dead_code)]
    tool: String,
    #[allow(dead_code)]
    path: Option<PathBuf>,
    version: Option<String>,
}

fn probe_cli_tool(i18n: &I18n, tool: &str) -> ToolProbe {
    let Ok(path) = which(tool) else {
        println!("{}", i18n.init_tool_not_found(tool));
        return ToolProbe {
            tool: tool.to_string(),
            path: None,
            version: None,
        };
    };

    let version = get_tool_version(&path, "--version");
    println!(
        "{}",
        i18n.init_tool_found(tool, &path.display().to_string(), version.as_deref())
    );

    ToolProbe {
        tool: tool.to_string(),
        path: Some(path),
        version,
    }
}

fn get_tool_version(path: &PathBuf, flag: &str) -> Option<String> {
    let output = Command::new(path).arg(flag).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let version_str = if stdout.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };

    version_str.lines().next().map(|s| s.to_string())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
}

impl SemVer {
    const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

fn parse_semver_from_text(text: &str) -> Option<SemVer> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let mut end = start;
    for (idx, c) in text[start..].char_indices() {
        if matches!(c, '0'..='9' | '.') {
            end = start + idx + c.len_utf8();
        } else {
            break;
        }
    }

    // Since find() guarantees start points to a digit, and the loop includes
    // that digit, raw will always contain at least one digit after trimming.
    let raw = text[start..end].trim_matches('.');

    let mut parts = raw.split('.');
    // split() always yields at least one element, even for empty string
    let major_str = parts
        .next()
        .expect("split always yields at least one element");
    let major = major_str.parse().ok()?;
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    Some(SemVer {
        major,
        minor,
        patch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    #[cfg(unix)]
    use crate::test_support::write_executable;
    use crate::test_support::{env_lock, CurrentDirGuard, EnvVarGuard};
    use std::fs;
    use tempfile::TempDir;

    fn test_i18n() -> I18n {
        I18n::new(Lang::En)
    }

    #[test]
    fn shnote_rules_has_content() {
        // Verify rules contain expected content
        assert!(SHNOTE_RULES_BASE.contains("shnote"));
        assert!(SHNOTE_RULES_BASE_EN.contains("shnote"));
        assert!(SHNOTE_RULES_BASE.contains("--what"));
        assert!(SHNOTE_RULES_BASE_EN.contains("--what"));
        assert!(SHNOTE_RULES_BASE.contains("--why"));
        assert!(SHNOTE_RULES_BASE_EN.contains("--why"));
        assert!(SHNOTE_RULES_BASE.contains("header_stream=auto"));
        assert!(SHNOTE_RULES_BASE_EN.contains("header_stream=auto"));
        assert!(SHNOTE_RULES_BASE.len() > 200);
        assert!(SHNOTE_RULES_BASE_EN.len() > 200);
    }

    #[test]
    fn codex_rules_include_extra_instruction() {
        let i18n = test_i18n();
        let rules = rules_for_target(&i18n, InitTarget::Codex);
        assert!(rules.contains("Read"));
        assert!(rules.contains("apply_patch"));
    }

    #[test]
    fn rules_do_not_include_pueue_section_when_available() {
        let i18n = test_i18n();
        let rules = rules_for_target_with_pueue(&i18n, InitTarget::Codex, true);
        assert!(!rules.contains("Long-running commands (use pueue)"));
    }

    #[test]
    fn rules_do_not_include_pueue_section_when_missing() {
        let i18n = test_i18n();
        let rules = rules_for_target_with_pueue(&i18n, InitTarget::Codex, false);
        assert!(!rules.contains("Long-running commands (use pueue)"));
    }

    #[test]
    fn markers_are_valid() {
        assert!(SHNOTE_MARKER_START.contains("shnote"));
        assert!(SHNOTE_MARKER_END.contains("shnote"));
    }

    #[test]
    fn parse_semver_from_text_parses_first_version_token() {
        assert_eq!(
            parse_semver_from_text("2.0.69 (Claude Code)"),
            Some(SemVer::new(2, 0, 69))
        );
        assert_eq!(
            parse_semver_from_text("codex-cli 0.72.0"),
            Some(SemVer::new(0, 72, 0))
        );
        assert_eq!(
            parse_semver_from_text("v2.0.64"),
            Some(SemVer::new(2, 0, 64))
        );
        assert_eq!(parse_semver_from_text("no version here"), None);
        // Test version string with only dots returns None (line 553)
        assert_eq!(parse_semver_from_text("..."), None);
        // Test version with number too large to parse as u32
        assert_eq!(parse_semver_from_text("99999999999999999999.0.0"), None);
    }

    #[cfg(unix)]
    #[test]
    fn get_tool_version_returns_none_on_nonzero_exit() {
        let temp_dir = TempDir::new().unwrap();
        let script = temp_dir.path().join("fail-tool");
        write_executable(&script, "#!/bin/sh\necho 'version 1.0.0'\nexit 1\n").unwrap();

        let result = get_tool_version(&script, "--version");
        assert!(result.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn get_tool_version_uses_stderr_when_stdout_empty() {
        let temp_dir = TempDir::new().unwrap();
        let script = temp_dir.path().join("stderr-tool");
        write_executable(&script, "#!/bin/sh\necho 'version 1.2.3' >&2\nexit 0\n").unwrap();

        let result = get_tool_version(&script, "--version");
        assert_eq!(result, Some("version 1.2.3".to_string()));
    }

    #[test]
    fn get_tool_version_returns_none_when_command_cannot_execute() {
        let result = get_tool_version(&PathBuf::from("/nonexistent/tool"), "--version");
        assert!(result.is_none());
    }

    #[test]
    fn append_rules_creates_new_file() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.md");

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        append_rules(&i18n, &target_file, &rules).unwrap();

        assert!(target_file.exists());
        let content = fs::read_to_string(&target_file).unwrap();
        assert!(content.contains(SHNOTE_MARKER_START));
        assert!(content.contains(SHNOTE_MARKER_END));
        assert!(content.contains("shnote"));
        assert!(!content.contains("{{NON_SHNOTE_TOOLS}}"));
    }

    #[test]
    fn append_rules_updates_existing() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.md");

        // Create file with old rules
        fs::write(
            &target_file,
            format!(
                "Some content\n{}OLD RULES{}\nMore content",
                SHNOTE_MARKER_START, SHNOTE_MARKER_END
            ),
        )
        .unwrap();

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        append_rules(&i18n, &target_file, &rules).unwrap();

        let content = fs::read_to_string(&target_file).unwrap();
        assert!(content.contains("Some content"));
        assert!(content.contains("More content"));
        assert!(!content.contains("OLD RULES"));
        assert!(content.contains(&rules));
    }

    #[cfg(unix)]
    #[test]
    fn init_claude_writes_rules_file_when_claude_is_new_enough() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());
        let tools_dir = TempDir::new().unwrap();
        let claude = tools_dir.path().join("claude");
        write_executable(&claude, "#!/bin/sh\necho \"Claude Code 2.0.64\"\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        let i18n = test_i18n();
        init_claude(&i18n, Scope::User).unwrap();

        let rules_file = temp_dir.path().join(".claude/rules/shnote.md");
        assert!(rules_file.exists());
        let content = fs::read_to_string(rules_file).unwrap();
        assert_eq!(content, rules_for_target(&i18n, InitTarget::Claude));
    }

    #[test]
    fn init_claude_appends_to_claude_md_when_claude_not_found() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());
        let tools_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        let i18n = test_i18n();
        init_claude(&i18n, Scope::User).unwrap();

        let target_file = temp_dir.path().join(".claude/CLAUDE.md");
        assert!(target_file.exists());
        let content = fs::read_to_string(target_file).unwrap();
        assert!(content.contains(SHNOTE_MARKER_START));
        assert!(content.contains(SHNOTE_MARKER_END));
        assert!(content.contains("shnote"));
    }

    #[cfg(unix)]
    #[test]
    fn init_claude_appends_to_claude_md_when_claude_is_old() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let tools_dir = TempDir::new().unwrap();
        let claude = tools_dir.path().join("claude");
        write_executable(&claude, "#!/bin/sh\necho \"2.0.63\"\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        let i18n = test_i18n();
        init_claude(&i18n, Scope::User).unwrap();

        let target_file = temp_dir.path().join(".claude/CLAUDE.md");
        assert!(target_file.exists());
        let content = fs::read_to_string(target_file).unwrap();
        assert!(content.contains(SHNOTE_MARKER_START));
        assert!(content.contains(SHNOTE_MARKER_END));
        assert!(content.contains("shnote"));
    }

    #[test]
    fn init_claude_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = test_i18n();
        let err = init_claude(&i18n, Scope::User).unwrap_err();
        assert!(err.to_string().contains(i18n.err_home_dir()));
    }

    #[cfg(unix)]
    #[test]
    fn init_claude_errors_when_create_dir_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());
        let tools_dir = TempDir::new().unwrap();
        let claude = tools_dir.path().join("claude");
        write_executable(&claude, "#!/bin/sh\necho \"2.0.64\"\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        // Make ~/.claude a file so ~/.claude/rules cannot be created.
        fs::write(temp_dir.path().join(".claude"), "not a dir").unwrap();

        let i18n = test_i18n();
        let err = init_claude(&i18n, Scope::User).unwrap_err();
        assert!(err.to_string().contains(
            &i18n.err_create_dir(&temp_dir.path().join(".claude/rules").display().to_string())
        ));
    }

    #[cfg(unix)]
    #[test]
    fn init_claude_errors_when_write_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());
        let tools_dir = TempDir::new().unwrap();
        let claude = tools_dir.path().join("claude");
        write_executable(&claude, "#!/bin/sh\necho \"2.0.64\"\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        fs::create_dir_all(temp_dir.path().join(".claude/rules/shnote.md")).unwrap();

        let i18n = test_i18n();
        let err = init_claude(&i18n, Scope::User).unwrap_err();
        assert!(err.to_string().contains(
            &i18n.err_write_file(
                &temp_dir
                    .path()
                    .join(".claude/rules/shnote.md")
                    .display()
                    .to_string()
            )
        ));
    }

    #[cfg(unix)]
    #[test]
    fn init_claude_errors_when_append_rules_fails_for_old_claude() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Simulate old claude version
        let tools_dir = TempDir::new().unwrap();
        let claude = tools_dir.path().join("claude");
        write_executable(&claude, "#!/bin/sh\necho \"2.0.63\"\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        // Make CLAUDE.md a directory so append_rules fails
        fs::create_dir_all(temp_dir.path().join(".claude/CLAUDE.md")).unwrap();

        let i18n = test_i18n();
        let err = init_claude(&i18n, Scope::User).unwrap_err();
        let err_debug = format!("{:?}", err);
        assert!(err_debug.contains("CLAUDE.md"));
    }

    #[cfg(unix)]
    #[test]
    fn init_claude_migrates_rules_from_old_claude_md() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Create old CLAUDE.md with shnote rules
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        let old_claude_md = claude_dir.join("CLAUDE.md");
        fs::write(
            &old_claude_md,
            format!(
                "# My Claude Config\n\nSome content\n{}OLD SHNOTE RULES{}\n\nMore content",
                SHNOTE_MARKER_START, SHNOTE_MARKER_END
            ),
        )
        .unwrap();

        // Simulate new claude version
        let tools_dir = TempDir::new().unwrap();
        let claude = tools_dir.path().join("claude");
        write_executable(&claude, "#!/bin/sh\necho \"Claude Code 2.0.64\"\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        let i18n = test_i18n();
        init_claude(&i18n, Scope::User).unwrap();

        // Check new rules file exists with latest content
        let rules_file = temp_dir.path().join(".claude/rules/shnote.md");
        assert!(rules_file.exists());
        let content = fs::read_to_string(&rules_file).unwrap();
        assert_eq!(content, rules_for_target(&i18n, InitTarget::Claude));

        // Check old CLAUDE.md no longer has shnote rules
        let old_content = fs::read_to_string(&old_claude_md).unwrap();
        assert!(!old_content.contains(SHNOTE_MARKER_START));
        assert!(!old_content.contains("OLD SHNOTE RULES"));
        assert!(old_content.contains("# My Claude Config"));
        assert!(old_content.contains("Some content"));
        assert!(old_content.contains("More content"));
    }

    #[cfg(unix)]
    #[test]
    fn init_claude_deletes_empty_claude_md_after_migration() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Create old CLAUDE.md with only shnote rules
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        let old_claude_md = claude_dir.join("CLAUDE.md");
        fs::write(
            &old_claude_md,
            format!(
                "{}OLD SHNOTE RULES{}",
                SHNOTE_MARKER_START, SHNOTE_MARKER_END
            ),
        )
        .unwrap();

        // Simulate new claude version
        let tools_dir = TempDir::new().unwrap();
        let claude = tools_dir.path().join("claude");
        write_executable(&claude, "#!/bin/sh\necho \"Claude Code 2.0.64\"\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        let i18n = test_i18n();
        init_claude(&i18n, Scope::User).unwrap();

        // Check new rules file exists
        let rules_file = temp_dir.path().join(".claude/rules/shnote.md");
        assert!(rules_file.exists());

        // Check old CLAUDE.md was deleted (it would be empty)
        assert!(!old_claude_md.exists());
    }

    #[cfg(unix)]
    #[test]
    fn init_claude_no_migration_when_old_claude_md_has_no_shnote() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Create old CLAUDE.md without shnote rules
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        let old_claude_md = claude_dir.join("CLAUDE.md");
        fs::write(&old_claude_md, "# My Claude Config\n\nSome other content").unwrap();

        // Simulate new claude version
        let tools_dir = TempDir::new().unwrap();
        let claude = tools_dir.path().join("claude");
        write_executable(&claude, "#!/bin/sh\necho \"Claude Code 2.0.64\"\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        let i18n = test_i18n();
        init_claude(&i18n, Scope::User).unwrap();

        // Check new rules file exists with latest content
        let rules_file = temp_dir.path().join(".claude/rules/shnote.md");
        assert!(rules_file.exists());
        let content = fs::read_to_string(&rules_file).unwrap();
        assert_eq!(content, rules_for_target(&i18n, InitTarget::Claude));

        // Check old CLAUDE.md is unchanged
        let old_content = fs::read_to_string(&old_claude_md).unwrap();
        assert_eq!(old_content, "# My Claude Config\n\nSome other content");
    }

    #[test]
    fn migrate_shnote_rules_returns_false_when_no_markers() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let old_file = temp_dir.path().join("old.md");
        let new_file = temp_dir.path().join("new.md");

        fs::write(&old_file, "Some content without markers").unwrap();

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        let migrated = migrate_shnote_rules(&i18n, &old_file, &new_file, &rules).unwrap();
        assert!(!migrated);
        assert!(!new_file.exists());
    }

    #[test]
    fn migrate_shnote_rules_handles_missing_end_marker() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let old_file = temp_dir.path().join("old.md");
        let new_file = temp_dir.path().join("new.md");

        // Missing end marker - should extract until end of file
        fs::write(
            &old_file,
            format!("Before{}OLD RULES WITHOUT END", SHNOTE_MARKER_START),
        )
        .unwrap();

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        let migrated = migrate_shnote_rules(&i18n, &old_file, &new_file, &rules).unwrap();
        assert!(migrated);
        assert!(new_file.exists());

        // Old file should have only "Before" (trimmed)
        let old_content = fs::read_to_string(&old_file).unwrap();
        assert_eq!(old_content, "Before");
    }

    #[test]
    fn migrate_shnote_rules_errors_when_read_fails() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let old_file = temp_dir.path().join("nonexistent.md");
        let new_file = temp_dir.path().join("new.md");

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        let err = migrate_shnote_rules(&i18n, &old_file, &new_file, &rules).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_read_file(&old_file.display().to_string())));
    }

    #[test]
    fn migrate_shnote_rules_errors_when_write_fails() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let old_file = temp_dir.path().join("old.md");
        // Make new_file a directory so write fails
        let new_file = temp_dir.path().join("new.md");
        fs::create_dir_all(&new_file).unwrap();

        fs::write(
            &old_file,
            format!("Before{}RULES{}", SHNOTE_MARKER_START, SHNOTE_MARKER_END),
        )
        .unwrap();

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        let err = migrate_shnote_rules(&i18n, &old_file, &new_file, &rules).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_write_file(&new_file.display().to_string())));
    }

    #[test]
    fn init_codex_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = test_i18n();
        let err = init_codex(&i18n, Scope::User).unwrap_err();
        assert!(err.to_string().contains(i18n.err_home_dir()));
    }

    #[test]
    fn init_gemini_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = test_i18n();
        let err = init_gemini(&i18n, Scope::User).unwrap_err();
        assert!(err.to_string().contains(i18n.err_home_dir()));
    }

    #[test]
    fn init_codex_errors_when_create_dir_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Make ~/.codex a file so ~/.codex cannot be created.
        fs::write(temp_dir.path().join(".codex"), "not a dir").unwrap();

        let i18n = test_i18n();
        let err = init_codex(&i18n, Scope::User).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_create_dir(&temp_dir.path().join(".codex").display().to_string())));
    }

    #[test]
    fn init_gemini_errors_when_create_dir_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Make ~/.gemini a file so ~/.gemini cannot be created.
        fs::write(temp_dir.path().join(".gemini"), "not a dir").unwrap();

        let i18n = test_i18n();
        let err = init_gemini(&i18n, Scope::User).unwrap_err();
        assert!(err.to_string().contains(
            &i18n.err_create_dir(&temp_dir.path().join(".gemini").display().to_string())
        ));
    }

    #[test]
    fn init_codex_errors_when_append_rules_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        fs::create_dir_all(temp_dir.path().join(".codex/AGENTS.md")).unwrap();

        let i18n = test_i18n();
        let err = init_codex(&i18n, Scope::User).unwrap_err();
        // Check error chain contains the read error context (use Debug format to see full chain)
        let err_debug = format!("{:?}", err);
        assert!(err_debug.contains("AGENTS.md"));
    }

    #[test]
    fn init_gemini_errors_when_append_rules_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        fs::create_dir_all(temp_dir.path().join(".gemini/GEMINI.md")).unwrap();

        let i18n = test_i18n();
        let err = init_gemini(&i18n, Scope::User).unwrap_err();
        // Check error chain contains the read error context (use Debug format to see full chain)
        let err_debug = format!("{:?}", err);
        assert!(err_debug.contains("GEMINI.md"));
    }

    #[test]
    fn append_rules_replaces_until_end_when_end_marker_missing() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.md");

        fs::write(
            &target_file,
            format!("before\n{SHNOTE_MARKER_START}OLD RULES WITHOUT END\nafter\n"),
        )
        .unwrap();

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        append_rules(&i18n, &target_file, &rules).unwrap();

        let content = fs::read_to_string(&target_file).unwrap();
        assert!(content.contains("before"));
        assert!(content.contains(&rules));
        assert!(!content.contains("OLD RULES WITHOUT END"));
        assert!(!content.contains("after"));
    }

    #[test]
    fn append_rules_errors_when_read_fails() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("dir-as-file");
        fs::create_dir_all(&target_file).unwrap();

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        let err = append_rules(&i18n, &target_file, &rules).unwrap_err();
        // Check error chain contains the file path (use Debug format to see full chain)
        let err_debug = format!("{:?}", err);
        assert!(err_debug.contains("dir-as-file"));
    }

    #[cfg(unix)]
    #[test]
    fn append_rules_errors_when_write_fails() {
        use std::os::unix::fs::PermissionsExt;

        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.md");

        fs::write(
            &target_file,
            format!(
                "Some content\n{}OLD RULES{}\nMore content",
                SHNOTE_MARKER_START, SHNOTE_MARKER_END
            ),
        )
        .unwrap();
        fs::set_permissions(&target_file, fs::Permissions::from_mode(0o444)).unwrap();

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        let err = append_rules(&i18n, &target_file, &rules).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_write_file(&target_file.display().to_string())));
    }

    #[cfg(unix)]
    #[test]
    fn append_rules_errors_when_append_write_fails() {
        use std::os::unix::fs::PermissionsExt;

        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.md");

        fs::write(&target_file, "existing content\n").unwrap();
        fs::set_permissions(&target_file, fs::Permissions::from_mode(0o444)).unwrap();

        let rules = rules_for_target(&i18n, InitTarget::Codex);
        let err = append_rules(&i18n, &target_file, &rules).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_write_file(&target_file.display().to_string())));
    }

    // Project scope tests
    #[test]
    fn init_claude_project_scope_writes_to_claude_md_when_claude_not_found() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();

        // Change to temp directory using RAII guard
        let _cwd_guard = CurrentDirGuard::set(temp_dir.path()).unwrap();

        // Mock PATH to not find claude (so it falls back to CLAUDE.md)
        let empty_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_dir.path());

        let i18n = test_i18n();
        init_claude(&i18n, Scope::Project).unwrap();

        // Check that rules were written to project directory
        let target_file = temp_dir.path().join(".claude/CLAUDE.md");
        assert!(target_file.exists());
        let content = fs::read_to_string(target_file).unwrap();
        assert!(content.contains(SHNOTE_MARKER_START));
        assert!(content.contains("shnote"));
    }

    #[cfg(unix)]
    #[test]
    fn init_claude_project_scope_writes_rules_when_claude_new_enough() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();

        let _cwd_guard = CurrentDirGuard::set(temp_dir.path()).unwrap();

        // Create a mock claude binary that returns version >= 2.0.64
        let tools_dir = TempDir::new().unwrap();
        let claude = tools_dir.path().join("claude");
        write_executable(&claude, "#!/bin/sh\necho \"Claude Code 2.0.64\"\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        let i18n = test_i18n();
        init_claude(&i18n, Scope::Project).unwrap();

        // Check that rules were written to rules directory
        let rules_file = temp_dir.path().join(".claude/rules/shnote.md");
        assert!(rules_file.exists());
        let content = fs::read_to_string(rules_file).unwrap();
        assert_eq!(content, rules_for_target(&i18n, InitTarget::Claude));

        // CLAUDE.md should not exist
        let claude_md = temp_dir.path().join(".claude/CLAUDE.md");
        assert!(!claude_md.exists());
    }

    #[test]
    fn init_codex_project_scope_writes_to_current_dir() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();

        let _cwd_guard = CurrentDirGuard::set(temp_dir.path()).unwrap();

        // Mock PATH to not find codex
        let empty_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_dir.path());

        let i18n = test_i18n();
        init_codex(&i18n, Scope::Project).unwrap();

        let target_file = temp_dir.path().join(".codex/AGENTS.md");
        assert!(target_file.exists());
        let content = fs::read_to_string(target_file).unwrap();
        assert!(content.contains(SHNOTE_MARKER_START));
        assert!(content.contains("shnote"));
        assert!(content.contains("apply_patch"));
    }

    #[test]
    fn init_gemini_project_scope_writes_to_current_dir() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();

        let _cwd_guard = CurrentDirGuard::set(temp_dir.path()).unwrap();

        // Mock PATH to not find gemini
        let empty_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_dir.path());

        let i18n = test_i18n();
        init_gemini(&i18n, Scope::Project).unwrap();

        let target_file = temp_dir.path().join(".gemini/GEMINI.md");
        assert!(target_file.exists());
        let content = fs::read_to_string(target_file).unwrap();
        assert!(content.contains(SHNOTE_MARKER_START));
        assert!(content.contains("shnote"));
    }

    #[test]
    fn get_base_dir_user_returns_home() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let i18n = test_i18n();
        let base = get_base_dir(&i18n, Scope::User).unwrap();
        assert_eq!(base, temp_dir.path());
    }

    #[test]
    fn get_base_dir_project_returns_current_dir() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();

        let _cwd_guard = CurrentDirGuard::set(temp_dir.path()).unwrap();

        let i18n = test_i18n();
        let base = get_base_dir(&i18n, Scope::Project).unwrap();

        // Use canonicalize to handle symlinks (e.g., /var -> /private/var on macOS)
        assert_eq!(
            base.canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }
}
