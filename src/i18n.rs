use std::env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
}

impl Lang {
    pub fn from_tag(tag: &str) -> Option<Self> {
        let raw = tag.trim();
        if raw.is_empty() {
            return None;
        }

        let raw = raw
            .split_once('.')
            .map(|(a, _)| a)
            .unwrap_or(raw)
            .replace('_', "-")
            .to_lowercase();

        // C/POSIX is not a real language, skip it to allow fallback
        if matches!(raw.as_str(), "c" | "posix") {
            return None;
        }

        if raw.starts_with("zh") {
            return Some(Self::Zh);
        }
        if raw.starts_with("en") {
            return Some(Self::En);
        }
        None
    }
}

pub struct I18n {
    lang: Lang,
}

impl I18n {
    pub fn new(lang: Lang) -> Self {
        Self { lang }
    }

    // CLI messages
    pub fn err_missing_what_why(&self, cmd: &str) -> String {
        match self.lang {
            Lang::En => format!(
                "`{cmd}` requires `--what` and `--why`, and they must appear before the subcommand.\n\
                Example: shnote --what \"...\" --why \"...\" {cmd} ..."
            ),
            Lang::Zh => format!(
                "`{cmd}` 需要 `--what` 和 `--why`，并且必须写在子命令之前。\n\
                示例：shnote --what \"...\" --why \"...\" {cmd} ..."
            ),
        }
    }

    pub fn err_reject_root_meta(&self) -> &'static str {
        match self.lang {
            Lang::En => "`--what/--why` are only accepted for `run`, `py`, `node`, `pip`, `npm`, and `npx` commands",
            Lang::Zh => "`--what/--why` 只允许用于 `run`、`py`、`node`、`pip`、`npm` 和 `npx` 命令",
        }
    }

    pub fn err_script_source_required(&self) -> &'static str {
        match self.lang {
            Lang::En => "exactly one of --stdin, -c/--code, -f/--file is required",
            Lang::Zh => "必须且只能指定一种脚本来源：--stdin、-c/--code、-f/--file",
        }
    }

    pub fn err_failed_to_execute(&self, cmd: &str) -> String {
        match self.lang {
            Lang::En => format!("failed to execute: {cmd}"),
            Lang::Zh => format!("执行失败：{cmd}"),
        }
    }

    pub fn err_interpreter_not_found(&self, name: &str) -> String {
        match self.lang {
            Lang::En => format!("interpreter not found: {name}"),
            Lang::Zh => format!("未找到解释器：{name}"),
        }
    }

    // Config messages
    pub fn config_key_not_found(&self, key: &str) -> String {
        match self.lang {
            Lang::En => format!("unknown config key: {key}"),
            Lang::Zh => format!("未知的配置项：{key}"),
        }
    }

    pub fn config_updated(&self, key: &str, value: &str) -> String {
        match self.lang {
            Lang::En => format!("config updated: {key} = {value}"),
            Lang::Zh => format!("配置已更新：{key} = {value}"),
        }
    }

    pub fn config_reset_done(&self) -> &'static str {
        match self.lang {
            Lang::En => "configuration reset to defaults",
            Lang::Zh => "配置已重置为默认值",
        }
    }

    // Doctor messages
    pub fn doctor_all_ok(&self) -> &'static str {
        match self.lang {
            Lang::En => "All dependencies OK!",
            Lang::Zh => "所有依赖检查通过！",
        }
    }

    pub fn doctor_has_issues(&self) -> &'static str {
        match self.lang {
            Lang::En => "Some dependencies have issues. Please fix them before using shnote.",
            Lang::Zh => "部分依赖存在问题，请先修复后再使用 shnote。",
        }
    }

    // Setup messages
    pub fn setup_starting(&self) -> &'static str {
        match self.lang {
            Lang::En => "Setting up shnote...",
            Lang::Zh => "正在设置 shnote...",
        }
    }

    pub fn setup_extracting(&self) -> &'static str {
        match self.lang {
            Lang::En => "Extracting embedded binaries...",
            Lang::Zh => "正在解压内嵌二进制文件...",
        }
    }

    pub fn setup_downloading(&self) -> &'static str {
        match self.lang {
            Lang::En => "Downloading pueue binaries...",
            Lang::Zh => "正在下载 pueue 二进制文件...",
        }
    }

    pub fn setup_path_instruction(&self) -> &'static str {
        match self.lang {
            Lang::En => "To use pueue, add the following to your PATH:",
            Lang::Zh => "要使用 pueue，请将以下路径添加到 PATH：",
        }
    }

    pub fn setup_complete(&self) -> &'static str {
        match self.lang {
            Lang::En => "Setup complete! Run `shnote doctor` to verify.",
            Lang::Zh => "设置完成！运行 `shnote doctor` 验证。",
        }
    }

    // Executor messages
    pub fn err_read_stdin(&self) -> &'static str {
        match self.lang {
            Lang::En => "failed to read from stdin",
            Lang::Zh => "从 stdin 读取失败",
        }
    }

    // Shell messages (Unix-specific methods may not be used on Windows and vice versa)
    #[cfg_attr(windows, allow(dead_code))]
    pub fn err_no_shell_unix(&self) -> &'static str {
        match self.lang {
            Lang::En => "no shell found in PATH (tried: zsh, bash, sh)",
            Lang::Zh => "在 PATH 中未找到 shell（已尝试：zsh、bash、sh）",
        }
    }

    #[cfg_attr(unix, allow(dead_code))]
    pub fn err_no_shell_windows(&self) -> &'static str {
        match self.lang {
            Lang::En => "no shell found (tried: pwsh, powershell, cmd)",
            Lang::Zh => "未找到 shell（已尝试：pwsh、powershell、cmd）",
        }
    }

    pub fn err_shell_not_in_path(&self, name: &str) -> String {
        match self.lang {
            Lang::En => format!("shell not found in PATH: {name}"),
            Lang::Zh => format!("在 PATH 中未找到 shell：{name}"),
        }
    }

    // Config error messages (some only used in specific code paths)
    #[allow(dead_code)]
    pub fn err_read_config(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("failed to read config file: {path}"),
            Lang::Zh => format!("读取配置文件失败：{path}"),
        }
    }

    #[allow(dead_code)]
    pub fn err_parse_config(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("failed to parse config file: {path}"),
            Lang::Zh => format!("解析配置文件失败：{path}"),
        }
    }

    pub fn err_create_config_dir(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("failed to create config directory: {path}"),
            Lang::Zh => format!("创建配置目录失败：{path}"),
        }
    }

    pub fn err_serialize_config(&self) -> &'static str {
        match self.lang {
            Lang::En => "failed to serialize config",
            Lang::Zh => "序列化配置失败",
        }
    }

    pub fn err_write_config(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("failed to write config file: {path}"),
            Lang::Zh => format!("写入配置文件失败：{path}"),
        }
    }

    pub fn err_invalid_shell_value(&self, value: &str, valid: &str) -> String {
        match self.lang {
            Lang::En => format!("invalid shell value: {value}. Valid options: {valid}"),
            Lang::Zh => format!("无效的 shell 值：{value}。有效选项：{valid}"),
        }
    }

    pub fn err_invalid_language_value(&self, value: &str, valid: &str) -> String {
        match self.lang {
            Lang::En => format!("invalid language value: {value}. Valid options: {valid}"),
            Lang::Zh => format!("无效的语言值：{value}。有效选项：{valid}"),
        }
    }

    #[allow(dead_code)]
    pub fn err_home_dir(&self) -> &'static str {
        match self.lang {
            Lang::En => "failed to determine home directory",
            Lang::Zh => "无法确定主目录",
        }
    }

    // Doctor error messages
    pub fn doctor_not_found_in_path(&self) -> &'static str {
        match self.lang {
            Lang::En => "not found in PATH",
            Lang::Zh => "在 PATH 中未找到",
        }
    }

    pub fn doctor_pueue_not_found(&self) -> &'static str {
        match self.lang {
            Lang::En => "not found (run `shnote setup` to install)",
            Lang::Zh => "未找到（运行 `shnote setup` 安装）",
        }
    }

    // Setup/download error messages
    pub fn err_create_dir(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("failed to create directory: {path}"),
            Lang::Zh => format!("创建目录失败：{path}"),
        }
    }

    pub fn err_download_failed(&self) -> &'static str {
        match self.lang {
            Lang::En => "download failed",
            Lang::Zh => "下载失败",
        }
    }

    pub fn err_download_no_tool(&self) -> &'static str {
        match self.lang {
            Lang::En => "failed to download (neither curl nor wget available)",
            Lang::Zh => "下载失败（curl 和 wget 都不可用）",
        }
    }

    #[cfg_attr(unix, allow(dead_code))]
    pub fn err_download_powershell(&self) -> &'static str {
        match self.lang {
            Lang::En => "failed to download using PowerShell",
            Lang::Zh => "使用 PowerShell 下载失败",
        }
    }

    pub fn err_checksum_mismatch(&self, path: &str, expected: &str, actual: &str) -> String {
        match self.lang {
            Lang::En => format!(
                "SHA256 checksum mismatch for {path}\n  expected: {expected}\n  actual:   {actual}"
            ),
            Lang::Zh => format!("{path} 的 SHA256 校验失败\n  预期：{expected}\n  实际：{actual}"),
        }
    }

    pub fn err_shasum_run(&self) -> &'static str {
        match self.lang {
            Lang::En => "failed to run shasum",
            Lang::Zh => "运行 shasum 失败",
        }
    }

    pub fn err_shasum_failed(&self) -> &'static str {
        match self.lang {
            Lang::En => "shasum failed",
            Lang::Zh => "shasum 执行失败",
        }
    }

    pub fn err_shasum_parse(&self) -> &'static str {
        match self.lang {
            Lang::En => "failed to parse shasum output",
            Lang::Zh => "解析 shasum 输出失败",
        }
    }

    #[cfg_attr(unix, allow(dead_code))]
    pub fn err_certutil_run(&self) -> &'static str {
        match self.lang {
            Lang::En => "failed to run certutil",
            Lang::Zh => "运行 certutil 失败",
        }
    }

    #[cfg_attr(unix, allow(dead_code))]
    pub fn err_certutil_failed(&self) -> &'static str {
        match self.lang {
            Lang::En => "certutil failed",
            Lang::Zh => "certutil 执行失败",
        }
    }

    #[cfg_attr(unix, allow(dead_code))]
    pub fn err_certutil_parse(&self) -> &'static str {
        match self.lang {
            Lang::En => "failed to parse certutil output",
            Lang::Zh => "解析 certutil 输出失败",
        }
    }

    pub fn err_create_file(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("failed to create file: {path}"),
            Lang::Zh => format!("创建文件失败：{path}"),
        }
    }

    pub fn err_write_file(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("failed to write file: {path}"),
            Lang::Zh => format!("写入文件失败：{path}"),
        }
    }

    pub fn err_read_file(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("failed to read file: {path}"),
            Lang::Zh => format!("读取文件失败：{path}"),
        }
    }

    // Init messages
    pub fn init_claude_success(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("✓ shnote rules installed to: {path}"),
            Lang::Zh => format!("✓ shnote 规则已安装到：{path}"),
        }
    }

    pub fn init_codex_success(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("✓ shnote rules written to: {path}"),
            Lang::Zh => format!("✓ shnote 规则已写入到：{path}"),
        }
    }

    pub fn init_gemini_success(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("✓ shnote rules written to: {path}"),
            Lang::Zh => format!("✓ shnote 规则已写入到：{path}"),
        }
    }

    pub fn init_rules_updated(&self) -> &'static str {
        match self.lang {
            Lang::En => "  (existing shnote rules were updated)",
            Lang::Zh => "  （已更新现有的 shnote 规则）",
        }
    }

    pub fn init_rules_appended(&self) -> &'static str {
        match self.lang {
            Lang::En => "  (rules appended to file)",
            Lang::Zh => "  （规则已追加到文件）",
        }
    }

    pub fn init_migrated_from(&self, old_path: &str) -> String {
        match self.lang {
            Lang::En => format!("  (migrated from {old_path})"),
            Lang::Zh => format!("  （已从 {old_path} 迁移）"),
        }
    }

    pub fn init_old_rules_cleaned(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("  (removed old rules from {path})"),
            Lang::Zh => format!("  （已从 {path} 移除旧规则）"),
        }
    }

    pub fn init_tool_found(&self, tool: &str, path: &str, version: Option<&str>) -> String {
        let version_str = version.map(|v| format!(" {v}")).unwrap_or_default();
        match self.lang {
            Lang::En => format!("✓ Detected {tool}:{version_str} ({path})"),
            Lang::Zh => format!("✓ 检测到 {tool}:{version_str}（{path}）"),
        }
    }

    pub fn init_tool_not_found(&self, tool: &str) -> String {
        match self.lang {
            Lang::En => format!("! {tool} not found in PATH (rules will still be written)"),
            Lang::Zh => format!("! 未在 PATH 中找到 {tool}（仍会写入规则）"),
        }
    }

    // === Help text translations (for clap runtime i18n) ===

    // App level
    pub fn help_app_about(&self) -> &'static str {
        match self.lang {
            Lang::En => "A lightweight command wrapper that enforces WHAT/WHY documentation",
            Lang::Zh => "轻量级命令包装器，强制执行 WHAT/WHY 文档记录",
        }
    }

    // Global arguments
    pub fn help_arg_what(&self) -> &'static str {
        match self.lang {
            Lang::En => "What this task does (required for run/py/node/pip/npm/npx, must appear before subcommand)",
            Lang::Zh => "这个任务做什么（run/py/node/pip/npm/npx 必需，必须在子命令之前）",
        }
    }

    pub fn help_arg_why(&self) -> &'static str {
        match self.lang {
            Lang::En => "Why this task is being executed (required for run/py/node/pip/npm/npx, must appear before subcommand)",
            Lang::Zh => "为什么执行这个任务（run/py/node/pip/npm/npx 必需，必须在子命令之前）",
        }
    }

    pub fn help_arg_lang(&self) -> &'static str {
        match self.lang {
            Lang::En => "Language for messages (auto-detected by default)",
            Lang::Zh => "消息语言（默认自动检测）",
        }
    }

    // Subcommands
    pub fn help_cmd_run(&self) -> &'static str {
        match self.lang {
            Lang::En => "Execute a shell command (passthrough)",
            Lang::Zh => "执行 shell 命令（透传）",
        }
    }

    pub fn help_cmd_py(&self) -> &'static str {
        match self.lang {
            Lang::En => "Execute a Python script",
            Lang::Zh => "执行 Python 脚本",
        }
    }

    pub fn help_cmd_node(&self) -> &'static str {
        match self.lang {
            Lang::En => "Execute a Node.js script",
            Lang::Zh => "执行 Node.js 脚本",
        }
    }

    pub fn help_cmd_pip(&self) -> &'static str {
        match self.lang {
            Lang::En => "Execute pip (Python package manager)",
            Lang::Zh => "执行 pip（Python 包管理器）",
        }
    }

    pub fn help_cmd_npm(&self) -> &'static str {
        match self.lang {
            Lang::En => "Execute npm (Node.js package manager)",
            Lang::Zh => "执行 npm（Node.js 包管理器）",
        }
    }

    pub fn help_cmd_npx(&self) -> &'static str {
        match self.lang {
            Lang::En => "Execute npx (Node.js package runner)",
            Lang::Zh => "执行 npx（Node.js 包运行器）",
        }
    }

    pub fn help_cmd_config(&self) -> &'static str {
        match self.lang {
            Lang::En => "Manage configuration",
            Lang::Zh => "管理配置",
        }
    }

    pub fn help_cmd_init(&self) -> &'static str {
        match self.lang {
            Lang::En => "Initialize shnote rules for AI tools",
            Lang::Zh => "为 AI 工具初始化 shnote 规则",
        }
    }

    pub fn help_cmd_setup(&self) -> &'static str {
        match self.lang {
            Lang::En => "Initialize environment (extract pueue binaries, etc.)",
            Lang::Zh => "初始化环境（解压 pueue 二进制文件等）",
        }
    }

    pub fn help_cmd_doctor(&self) -> &'static str {
        match self.lang {
            Lang::En => "Check environment dependencies (python/node/pueue)",
            Lang::Zh => "检查环境依赖（python/node/pueue）",
        }
    }

    pub fn help_cmd_completions(&self) -> &'static str {
        match self.lang {
            Lang::En => "Generate shell completion scripts",
            Lang::Zh => "生成 shell 补全脚本",
        }
    }

    // Config subcommands
    pub fn help_cmd_config_get(&self) -> &'static str {
        match self.lang {
            Lang::En => "Get a configuration value",
            Lang::Zh => "获取配置值",
        }
    }

    pub fn help_cmd_config_set(&self) -> &'static str {
        match self.lang {
            Lang::En => "Set a configuration value",
            Lang::Zh => "设置配置值",
        }
    }

    pub fn help_cmd_config_list(&self) -> &'static str {
        match self.lang {
            Lang::En => "List all configuration values",
            Lang::Zh => "列出所有配置值",
        }
    }

    pub fn help_cmd_config_reset(&self) -> &'static str {
        match self.lang {
            Lang::En => "Reset configuration to defaults",
            Lang::Zh => "重置配置为默认值",
        }
    }

    pub fn help_cmd_config_path(&self) -> &'static str {
        match self.lang {
            Lang::En => "Show configuration file path",
            Lang::Zh => "显示配置文件路径",
        }
    }

    // Init subcommands
    pub fn help_cmd_init_claude(&self) -> &'static str {
        match self.lang {
            Lang::En => "Install shnote rules for Claude Code (>= 2.0.64: ~/.claude/rules/shnote.md; otherwise: ~/.claude/CLAUDE.md)",
            Lang::Zh => "为 Claude Code 安装 shnote 规则（>= 2.0.64: ~/.claude/rules/shnote.md；否则: ~/.claude/CLAUDE.md）",
        }
    }

    pub fn help_cmd_init_codex(&self) -> &'static str {
        match self.lang {
            Lang::En => "Install or update shnote rules for Codex (~/.codex/AGENTS.md)",
            Lang::Zh => "为 Codex 安装或更新 shnote 规则（~/.codex/AGENTS.md）",
        }
    }

    pub fn help_cmd_init_gemini(&self) -> &'static str {
        match self.lang {
            Lang::En => "Install or update shnote rules for Gemini (~/.gemini/GEMINI.md)",
            Lang::Zh => "为 Gemini 安装或更新 shnote 规则（~/.gemini/GEMINI.md）",
        }
    }

    // Script args
    pub fn help_arg_code(&self) -> &'static str {
        match self.lang {
            Lang::En => "Inline script code",
            Lang::Zh => "内联脚本代码",
        }
    }

    pub fn help_arg_file(&self) -> &'static str {
        match self.lang {
            Lang::En => "Script file path",
            Lang::Zh => "脚本文件路径",
        }
    }

    pub fn help_arg_stdin(&self) -> &'static str {
        match self.lang {
            Lang::En => "Read script from stdin (supports heredoc)",
            Lang::Zh => "从 stdin 读取脚本（支持 heredoc）",
        }
    }

    pub fn help_arg_script_args(&self) -> &'static str {
        match self.lang {
            Lang::En => "Arguments passed to the script",
            Lang::Zh => "传递给脚本的参数",
        }
    }

    // Run/passthrough args
    pub fn help_arg_command(&self) -> &'static str {
        match self.lang {
            Lang::En => "Command and arguments to execute",
            Lang::Zh => "要执行的命令和参数",
        }
    }

    pub fn help_arg_passthrough(&self) -> &'static str {
        match self.lang {
            Lang::En => "Arguments to pass through to the underlying command",
            Lang::Zh => "传递给底层命令的参数",
        }
    }

    // Config args
    pub fn help_arg_config_key(&self) -> &'static str {
        match self.lang {
            Lang::En => "Configuration key (e.g., python, node, shell, language)",
            Lang::Zh => "配置键（如 python、node、shell、language）",
        }
    }

    pub fn help_arg_config_key_short(&self) -> &'static str {
        match self.lang {
            Lang::En => "Configuration key",
            Lang::Zh => "配置键",
        }
    }

    pub fn help_arg_config_value(&self) -> &'static str {
        match self.lang {
            Lang::En => "Configuration value",
            Lang::Zh => "配置值",
        }
    }

    // Completions args
    pub fn help_arg_shell(&self) -> &'static str {
        match self.lang {
            Lang::En => "Shell to generate completions for",
            Lang::Zh => "要生成补全脚本的 shell",
        }
    }
}

pub fn detect_lang(cli_lang: Option<&str>, config_lang: &str) -> Lang {
    // Priority: CLI flag > config > environment > default
    if let Some(lang) = cli_lang.and_then(Lang::from_tag) {
        return lang;
    }

    if config_lang != "auto" {
        if let Some(lang) = Lang::from_tag(config_lang) {
            return lang;
        }
    }

    parse_env_lang().unwrap_or(Lang::En)
}

fn parse_env_lang() -> Option<Lang> {
    let keys = ["SHNOTE_LANG", "LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"];
    for k in keys {
        let Some(v) = env::var_os(k) else { continue };
        let mut s = v.to_string_lossy().to_string();
        if k == "LANGUAGE" {
            if let Some((first, _)) = s.split_once(':') {
                s = first.to_string();
            }
        }
        if let Some(lang) = Lang::from_tag(&s) {
            return Some(lang);
        }
    }

    // Platform-specific detection
    #[cfg(target_os = "macos")]
    {
        if let Some(lang) = detect_macos_lang() {
            return Some(lang);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(lang) = detect_windows_lang() {
            return Some(lang);
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn detect_macos_lang() -> Option<Lang> {
    use std::process::Command;

    // Try AppleLocale first
    let output = Command::new("defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let locale = String::from_utf8_lossy(&output.stdout);
    Lang::from_tag(locale.trim())
}

#[cfg(target_os = "windows")]
fn detect_windows_lang() -> Option<Lang> {
    use std::process::Command;

    // Use PowerShell to get the current culture
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", "(Get-Culture).Name"])
        .output()
        .ok()?;

    if output.status.success() {
        let culture = String::from_utf8_lossy(&output.stdout);
        if let Some(lang) = Lang::from_tag(culture.trim()) {
            return Some(lang);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{env_lock, EnvVarGuard};

    #[cfg(target_os = "macos")]
    use crate::test_support::write_executable;
    #[cfg(target_os = "macos")]
    use tempfile::TempDir;

    #[test]
    fn lang_from_tag() {
        assert_eq!(Lang::from_tag("en"), Some(Lang::En));
        assert_eq!(Lang::from_tag("en_US"), Some(Lang::En));
        assert_eq!(Lang::from_tag("en_US.UTF-8"), Some(Lang::En));
        assert_eq!(Lang::from_tag("zh"), Some(Lang::Zh));
        assert_eq!(Lang::from_tag("zh_CN"), Some(Lang::Zh));
        assert_eq!(Lang::from_tag("zh-Hans"), Some(Lang::Zh));
        // C/POSIX should return None to allow fallback to system language
        assert_eq!(Lang::from_tag("C"), None);
        assert_eq!(Lang::from_tag("POSIX"), None);
        assert_eq!(Lang::from_tag("C.UTF-8"), None);
        assert_eq!(Lang::from_tag(""), None);
        assert_eq!(Lang::from_tag("fr"), None);
    }

    #[test]
    fn detect_lang_priority() {
        // CLI flag takes priority
        assert_eq!(detect_lang(Some("zh"), "en"), Lang::Zh);
        assert_eq!(detect_lang(Some("en"), "zh"), Lang::En);

        // Config takes priority over auto
        assert_eq!(detect_lang(None, "zh"), Lang::Zh);
        assert_eq!(detect_lang(None, "en"), Lang::En);

        // Auto falls back to environment/system/default.
        // Make it deterministic by controlling env vars to avoid partial coverage from `||`.
        let _lock = env_lock();
        let _prev_shnote_lang = EnvVarGuard::remove("SHNOTE_LANG");
        let _prev_lc_all = EnvVarGuard::remove("LC_ALL");
        let _prev_lc_messages = EnvVarGuard::remove("LC_MESSAGES");
        let _prev_language = EnvVarGuard::remove("LANGUAGE");
        let _prev_lang = EnvVarGuard::remove("LANG");

        let _language = EnvVarGuard::set("LANGUAGE", "zh:en");
        assert_eq!(detect_lang(None, "auto"), Lang::Zh);

        drop(_language);
        let _language = EnvVarGuard::set("LANGUAGE", "en:zh");
        assert_eq!(detect_lang(None, "auto"), Lang::En);
    }

    #[test]
    fn i18n_error_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        // Test various error messages
        assert!(en.err_missing_what_why("run").contains("--what"));
        assert!(zh.err_missing_what_why("run").contains("--what"));

        assert!(en.err_reject_root_meta().contains("--what"));
        assert!(zh.err_reject_root_meta().contains("--what"));

        assert!(en.err_script_source_required().contains("stdin"));
        assert!(zh.err_script_source_required().contains("stdin"));

        assert!(en.err_failed_to_execute("test").contains("test"));
        assert!(zh.err_failed_to_execute("test").contains("test"));

        assert!(en.err_interpreter_not_found("python").contains("python"));
        assert!(zh.err_interpreter_not_found("python").contains("python"));
    }

    #[test]
    fn i18n_config_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        assert!(en.config_key_not_found("foo").contains("foo"));
        assert!(zh.config_key_not_found("foo").contains("foo"));

        assert!(en.config_updated("key", "val").contains("key"));
        assert!(zh.config_updated("key", "val").contains("val"));

        assert!(!en.config_reset_done().is_empty());
        assert!(!zh.config_reset_done().is_empty());
    }

    #[test]
    fn i18n_doctor_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        assert!(!en.doctor_all_ok().is_empty());
        assert!(!zh.doctor_all_ok().is_empty());

        assert!(!en.doctor_has_issues().is_empty());
        assert!(!zh.doctor_has_issues().is_empty());

        assert!(!en.doctor_not_found_in_path().is_empty());
        assert!(!zh.doctor_not_found_in_path().is_empty());

        assert!(!en.doctor_pueue_not_found().is_empty());
        assert!(!zh.doctor_pueue_not_found().is_empty());
    }

    #[test]
    fn i18n_setup_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        assert!(!en.setup_starting().is_empty());
        assert!(!zh.setup_starting().is_empty());

        assert!(!en.setup_extracting().is_empty());
        assert!(!zh.setup_extracting().is_empty());

        assert!(!en.setup_downloading().is_empty());
        assert!(!zh.setup_downloading().is_empty());

        assert!(!en.setup_path_instruction().is_empty());
        assert!(!zh.setup_path_instruction().is_empty());

        assert!(!en.setup_complete().is_empty());
        assert!(!zh.setup_complete().is_empty());
    }

    #[test]
    fn i18n_file_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        assert!(en.err_create_dir("/tmp").contains("/tmp"));
        assert!(zh.err_create_dir("/tmp").contains("/tmp"));

        assert!(en.err_create_file("/tmp/f").contains("/tmp/f"));
        assert!(zh.err_create_file("/tmp/f").contains("/tmp/f"));

        assert!(en.err_write_file("/tmp/f").contains("/tmp/f"));
        assert!(zh.err_write_file("/tmp/f").contains("/tmp/f"));

        assert!(en.err_read_file("/tmp/f").contains("/tmp/f"));
        assert!(zh.err_read_file("/tmp/f").contains("/tmp/f"));
    }

    #[test]
    fn i18n_init_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        assert!(en.init_claude_success("/tmp/f").contains("/tmp/f"));
        assert!(zh.init_claude_success("/tmp/f").contains("/tmp/f"));

        assert!(en.init_codex_success("/tmp/f").contains("/tmp/f"));
        assert!(zh.init_codex_success("/tmp/f").contains("/tmp/f"));

        assert!(en.init_gemini_success("/tmp/f").contains("/tmp/f"));
        assert!(zh.init_gemini_success("/tmp/f").contains("/tmp/f"));

        assert!(!en.init_rules_updated().is_empty());
        assert!(!zh.init_rules_updated().is_empty());

        assert!(!en.init_rules_appended().is_empty());
        assert!(!zh.init_rules_appended().is_empty());

        assert!(en.init_migrated_from("/old/path").contains("/old/path"));
        assert!(zh.init_migrated_from("/old/path").contains("/old/path"));

        assert!(en.init_old_rules_cleaned("/old/path").contains("/old/path"));
        assert!(zh.init_old_rules_cleaned("/old/path").contains("/old/path"));

        assert!(en
            .init_tool_found("claude", "/tmp/claude", Some("Claude Code 2.0.64"))
            .contains("claude"));
        assert!(zh
            .init_tool_found("claude", "/tmp/claude", Some("Claude Code 2.0.64"))
            .contains("claude"));

        assert!(en.init_tool_not_found("claude").contains("claude"));
        assert!(zh.init_tool_not_found("claude").contains("claude"));
    }

    #[test]
    fn i18n_shell_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        #[cfg(unix)]
        {
            assert!(!en.err_no_shell_unix().is_empty());
            assert!(!zh.err_no_shell_unix().is_empty());
        }

        assert!(!en.err_no_shell_windows().is_empty());
        assert!(!zh.err_no_shell_windows().is_empty());

        assert!(en.err_shell_not_in_path("bash").contains("bash"));
        assert!(zh.err_shell_not_in_path("bash").contains("bash"));
    }

    #[test]
    fn i18n_pueue_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        assert!(en
            .err_checksum_mismatch("/tmp", "abc", "def")
            .contains("abc"));
        assert!(zh
            .err_checksum_mismatch("/tmp", "abc", "def")
            .contains("abc"));

        assert!(!en.err_shasum_run().is_empty());
        assert!(!zh.err_shasum_run().is_empty());
        assert!(!en.err_shasum_failed().is_empty());
        assert!(!zh.err_shasum_failed().is_empty());
        assert!(!en.err_shasum_parse().is_empty());
        assert!(!zh.err_shasum_parse().is_empty());

        assert!(!en.err_certutil_run().is_empty());
        assert!(!zh.err_certutil_run().is_empty());
        assert!(!en.err_certutil_failed().is_empty());
        assert!(!zh.err_certutil_failed().is_empty());
        assert!(!en.err_certutil_parse().is_empty());
        assert!(!zh.err_certutil_parse().is_empty());
    }

    #[test]
    fn i18n_executor_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        assert!(!en.err_read_stdin().is_empty());
        assert!(!zh.err_read_stdin().is_empty());

        assert!(!en.err_home_dir().is_empty());
        assert!(!zh.err_home_dir().is_empty());
    }

    #[test]
    fn i18n_download_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        assert!(!en.err_download_failed().is_empty());
        assert!(!zh.err_download_failed().is_empty());

        assert!(!en.err_download_no_tool().is_empty());
        assert!(!zh.err_download_no_tool().is_empty());

        assert!(!en.err_download_powershell().is_empty());
        assert!(!zh.err_download_powershell().is_empty());
    }

    #[test]
    fn i18n_config_error_messages() {
        let en = I18n::new(Lang::En);
        let zh = I18n::new(Lang::Zh);

        assert!(en.err_read_config("/tmp/c").contains("/tmp/c"));
        assert!(zh.err_read_config("/tmp/c").contains("/tmp/c"));

        assert!(en.err_parse_config("/tmp/c").contains("/tmp/c"));
        assert!(zh.err_parse_config("/tmp/c").contains("/tmp/c"));

        assert!(en.err_create_config_dir("/tmp/d").contains("/tmp/d"));
        assert!(zh.err_create_config_dir("/tmp/d").contains("/tmp/d"));

        assert!(en.err_write_config("/tmp/c").contains("/tmp/c"));
        assert!(zh.err_write_config("/tmp/c").contains("/tmp/c"));
    }

    #[test]
    fn parse_env_lang_prefers_language_key_and_splits_colon_list() {
        let _lock = env_lock();
        let _prev_shnote_lang = EnvVarGuard::remove("SHNOTE_LANG");
        let _prev_lc_all = EnvVarGuard::remove("LC_ALL");
        let _prev_lc_messages = EnvVarGuard::remove("LC_MESSAGES");
        let _prev_language = EnvVarGuard::remove("LANGUAGE");
        let _prev_lang = EnvVarGuard::remove("LANG");

        let _language = EnvVarGuard::set("LANGUAGE", "zh_CN:en_US");
        let _lang = EnvVarGuard::set("LANG", "zh_CN.UTF-8");
        let shnote_lang = EnvVarGuard::set("SHNOTE_LANG", "en_US.UTF-8");

        assert_eq!(parse_env_lang(), Some(Lang::En));

        drop(shnote_lang);
        assert_eq!(parse_env_lang(), Some(Lang::Zh));
    }

    #[test]
    fn parse_env_lang_accepts_language_without_colon() {
        let _lock = env_lock();
        let _prev_shnote_lang = EnvVarGuard::remove("SHNOTE_LANG");
        let _prev_lc_all = EnvVarGuard::remove("LC_ALL");
        let _prev_lc_messages = EnvVarGuard::remove("LC_MESSAGES");
        let _prev_language = EnvVarGuard::remove("LANGUAGE");
        let _prev_lang = EnvVarGuard::remove("LANG");

        let _language = EnvVarGuard::set("LANGUAGE", "en_US.UTF-8");
        assert_eq!(parse_env_lang(), Some(Lang::En));
    }

    #[test]
    fn detect_lang_ignores_invalid_config_value() {
        let _lock = env_lock();
        let _prev_shnote_lang = EnvVarGuard::remove("SHNOTE_LANG");
        let _prev_lc_all = EnvVarGuard::remove("LC_ALL");
        let _prev_lc_messages = EnvVarGuard::remove("LC_MESSAGES");
        let _prev_language = EnvVarGuard::remove("LANGUAGE");
        let _prev_lang = EnvVarGuard::remove("LANG");

        let _shnote_lang = EnvVarGuard::set("SHNOTE_LANG", "zh");
        assert_eq!(detect_lang(None, "invalid"), Lang::Zh);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_env_lang_uses_macos_defaults_when_env_missing() {
        let _lock = env_lock();

        let _shnote_lang = EnvVarGuard::remove("SHNOTE_LANG");
        let _lc_all = EnvVarGuard::remove("LC_ALL");
        let _lc_messages = EnvVarGuard::remove("LC_MESSAGES");
        let _language = EnvVarGuard::remove("LANGUAGE");
        let _lang = EnvVarGuard::remove("LANG");

        let temp_dir = TempDir::new().unwrap();
        let defaults = temp_dir.path().join("defaults");
        write_executable(&defaults, "#!/bin/sh\necho \"zh_CN\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());
        assert_eq!(parse_env_lang(), Some(Lang::Zh));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_env_lang_returns_none_when_macos_defaults_missing() {
        let _lock = env_lock();

        let _shnote_lang = EnvVarGuard::remove("SHNOTE_LANG");
        let _lc_all = EnvVarGuard::remove("LC_ALL");
        let _lc_messages = EnvVarGuard::remove("LC_MESSAGES");
        let _language = EnvVarGuard::remove("LANGUAGE");
        let _lang = EnvVarGuard::remove("LANG");

        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());
        assert_eq!(parse_env_lang(), None);
        assert_eq!(detect_lang(None, "auto"), Lang::En);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_env_lang_returns_none_when_macos_defaults_unrecognized() {
        let _lock = env_lock();

        let _shnote_lang = EnvVarGuard::remove("SHNOTE_LANG");
        let _lc_all = EnvVarGuard::remove("LC_ALL");
        let _lc_messages = EnvVarGuard::remove("LC_MESSAGES");
        let _language = EnvVarGuard::remove("LANGUAGE");
        let _lang = EnvVarGuard::remove("LANG");

        let temp_dir = TempDir::new().unwrap();
        let defaults = temp_dir.path().join("defaults");
        write_executable(&defaults, "#!/bin/sh\necho \"C\"\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());
        assert_eq!(parse_env_lang(), None);
        assert_eq!(detect_lang(None, "auto"), Lang::En);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_env_lang_returns_none_when_macos_defaults_fails() {
        let _lock = env_lock();

        let _shnote_lang = EnvVarGuard::remove("SHNOTE_LANG");
        let _lc_all = EnvVarGuard::remove("LC_ALL");
        let _lc_messages = EnvVarGuard::remove("LC_MESSAGES");
        let _language = EnvVarGuard::remove("LANGUAGE");
        let _lang = EnvVarGuard::remove("LANG");

        let temp_dir = TempDir::new().unwrap();
        let defaults = temp_dir.path().join("defaults");
        write_executable(&defaults, "#!/bin/sh\nexit 1\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());
        assert_eq!(parse_env_lang(), None);
        assert_eq!(detect_lang(None, "auto"), Lang::En);
    }
}
