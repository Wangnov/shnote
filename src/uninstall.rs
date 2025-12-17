use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::UninstallArgs;
use crate::config::{home_dir, shnote_home};
use crate::i18n::I18n;
use crate::info::get_install_path;

pub fn run_uninstall(i18n: &I18n, args: UninstallArgs) -> Result<()> {
    let install_path = get_install_path();
    let data_path = shnote_home().ok();

    // Show what will be removed
    println!("{}", i18n.uninstall_will_remove());
    println!();

    if let Some(path) = &install_path {
        println!("  - {}", path.display());
    }
    if let Some(path) = &data_path {
        if path.exists() {
            println!("  - {}/ ({})", path.display(), i18n.uninstall_config_data());
        }
    }
    println!();

    // Show manual removal hints
    println!("{}", i18n.uninstall_manual_removal());
    println!();
    println!("  - {}", i18n.uninstall_path_entry());

    // Check for AI rules files
    let ai_rules = find_ai_rules_files();
    if !ai_rules.is_empty() {
        println!("  - {}:", i18n.uninstall_ai_rules());
        for path in &ai_rules {
            println!("      {}", path.display());
        }
    }
    println!();

    // Confirm unless --yes
    if !args.yes {
        print!("{} [y/N] ", i18n.uninstall_confirm());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().lock().read_line(&mut input)?;

        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("{}", i18n.uninstall_cancelled());
            return Ok(());
        }
    }

    println!();

    // Remove data directory
    if let Some(path) = &data_path {
        if path.exists() {
            println!("{} {}...", i18n.uninstall_removing(), path.display());
            fs::remove_dir_all(path).context(i18n.uninstall_err_remove_data())?;
        }
    }

    // Remove binary (this should be done last since we're the running process)
    if let Some(path) = &install_path {
        println!("{} {}...", i18n.uninstall_removing(), path.display());

        #[cfg(unix)]
        {
            // On Unix, we can delete the running binary
            fs::remove_file(path).context(i18n.uninstall_err_remove_binary())?;
        }

        #[cfg(windows)]
        {
            // On Windows, we schedule deletion on exit
            // First try direct removal
            if let Err(_) = fs::remove_file(path) {
                // If that fails, rename to .old and hope it gets cleaned up
                let old_path = path.with_extension("exe.old.delete");
                let _ = fs::rename(path, &old_path);
                println!("  {}", i18n.uninstall_windows_note());
            }
        }
    }

    println!();
    println!("{}", i18n.uninstall_success());
    println!();
    println!("{}", i18n.uninstall_manual_steps());

    Ok(())
}

/// Find AI rules files that may contain shnote rules
fn find_ai_rules_files() -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(home) = home_dir() {
        // Claude Code rules
        let claude_rules = home.join(".claude/rules/shnote.md");
        if claude_rules.exists() {
            files.push(claude_rules);
        }

        let claude_md = home.join(".claude/CLAUDE.md");
        if claude_md.exists() && file_contains_shnote(&claude_md) {
            files.push(claude_md);
        }

        // Codex rules
        let codex_agents = home.join(".codex/AGENTS.md");
        if codex_agents.exists() && file_contains_shnote(&codex_agents) {
            files.push(codex_agents);
        }

        // Gemini rules
        let gemini_md = home.join(".gemini/GEMINI.md");
        if gemini_md.exists() && file_contains_shnote(&gemini_md) {
            files.push(gemini_md);
        }
    }

    files
}

fn file_contains_shnote(path: &PathBuf) -> bool {
    fs::read_to_string(path)
        .map(|content| content.contains("shnote"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{env_lock, EnvVarGuard};
    use tempfile::TempDir;

    #[test]
    fn find_ai_rules_files_returns_empty_when_no_files() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let files = find_ai_rules_files();
        assert!(files.is_empty());
    }

    #[test]
    fn find_ai_rules_files_finds_claude_rules() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Create Claude rules file
        let rules_dir = temp_dir.path().join(".claude/rules");
        fs::create_dir_all(&rules_dir).unwrap();
        fs::write(rules_dir.join("shnote.md"), "# shnote rules").unwrap();

        let files = find_ai_rules_files();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("shnote.md"));
    }

    #[test]
    fn find_ai_rules_files_finds_claude_md_with_shnote() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Create CLAUDE.md with shnote content
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join("CLAUDE.md"), "# shnote rules here").unwrap();

        let files = find_ai_rules_files();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("CLAUDE.md"));
    }

    #[test]
    fn find_ai_rules_files_ignores_claude_md_without_shnote() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Create CLAUDE.md without shnote content
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join("CLAUDE.md"), "# other rules").unwrap();

        let files = find_ai_rules_files();
        assert!(files.is_empty());
    }

    #[test]
    fn file_contains_shnote_returns_true_when_found() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.md");
        fs::write(&file, "# shnote rules\nsome content").unwrap();

        assert!(file_contains_shnote(&file));
    }

    #[test]
    fn file_contains_shnote_returns_false_when_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.md");
        fs::write(&file, "# other rules\nsome content").unwrap();

        assert!(!file_contains_shnote(&file));
    }

    #[test]
    fn file_contains_shnote_returns_false_when_file_missing() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("nonexistent.md");

        assert!(!file_contains_shnote(&file));
    }
}
