//! Runtime localization for clap Command help text.
//!
//! This module provides functions to localize clap's compile-time static help
//! strings at runtime based on the detected language.

use clap::Command;

use crate::i18n::I18n;

/// Localize all help text in a Command tree.
///
/// This function recursively walks through the command and all its subcommands,
/// replacing the help text with localized versions.
pub fn localize_command(cmd: Command, i18n: &I18n) -> Command {
    let name = cmd.get_name().to_string();

    // Collect subcommand names first to avoid borrowing issues
    let subcommand_names: Vec<String> = cmd
        .get_subcommands()
        .map(|sub| sub.get_name().to_string())
        .collect();

    // Start by localizing the current command's about text
    let mut cmd = cmd.about(get_command_about(&name, i18n));

    // Localize arguments
    cmd = localize_args(cmd, &name, i18n);

    // Recursively localize subcommands using mut_subcommand
    for sub_name in subcommand_names {
        cmd = cmd.mut_subcommand(&sub_name, |sub| localize_command(sub, i18n));
    }

    cmd
}

fn get_command_about(name: &str, i18n: &I18n) -> &'static str {
    match name {
        "shnote" => i18n.help_app_about(),
        "run" => i18n.help_cmd_run(),
        "py" => i18n.help_cmd_py(),
        "node" => i18n.help_cmd_node(),
        "pip" => i18n.help_cmd_pip(),
        "npm" => i18n.help_cmd_npm(),
        "npx" => i18n.help_cmd_npx(),
        "config" => i18n.help_cmd_config(),
        "init" => i18n.help_cmd_init(),
        "setup" => i18n.help_cmd_setup(),
        "doctor" => i18n.help_cmd_doctor(),
        "completions" => i18n.help_cmd_completions(),
        "info" => i18n.help_cmd_info(),
        "update" => i18n.help_cmd_update(),
        "uninstall" => i18n.help_cmd_uninstall(),
        // Config subcommands
        "get" => i18n.help_cmd_config_get(),
        "set" => i18n.help_cmd_config_set(),
        "list" => i18n.help_cmd_config_list(),
        "reset" => i18n.help_cmd_config_reset(),
        "path" => i18n.help_cmd_config_path(),
        // Init subcommands
        "claude" => i18n.help_cmd_init_claude(),
        "codex" => i18n.help_cmd_init_codex(),
        "gemini" => i18n.help_cmd_init_gemini(),
        _ => "", // Keep original for unknown commands
    }
}

fn localize_args(cmd: Command, cmd_name: &str, i18n: &I18n) -> Command {
    match cmd_name {
        "shnote" => cmd
            .mut_arg("what", |arg| arg.help(i18n.help_arg_what()))
            .mut_arg("why", |arg| arg.help(i18n.help_arg_why()))
            .mut_arg("lang", |arg| arg.help(i18n.help_arg_lang()))
            .mut_arg("header_stream", |arg| {
                arg.help(i18n.help_arg_header_stream())
            }),
        "run" => cmd.mut_arg("command", |arg| arg.help(i18n.help_arg_command())),
        "py" | "node" => cmd
            .mut_arg("code", |arg| arg.help(i18n.help_arg_code()))
            .mut_arg("file", |arg| arg.help(i18n.help_arg_file()))
            .mut_arg("stdin", |arg| arg.help(i18n.help_arg_stdin()))
            .mut_arg("args", |arg| arg.help(i18n.help_arg_script_args())),
        "pip" | "npm" | "npx" => cmd.mut_arg("args", |arg| arg.help(i18n.help_arg_passthrough())),
        "update" => cmd
            .mut_arg("check", |arg| arg.help(i18n.help_arg_update_check()))
            .mut_arg("force", |arg| arg.help(i18n.help_arg_update_force())),
        "uninstall" => cmd.mut_arg("yes", |arg| arg.help(i18n.help_arg_uninstall_yes())),
        "get" => cmd.mut_arg("key", |arg| arg.help(i18n.help_arg_config_key())),
        "set" => cmd
            .mut_arg("key", |arg| arg.help(i18n.help_arg_config_key_short()))
            .mut_arg("value", |arg| arg.help(i18n.help_arg_config_value())),
        "completions" => cmd.mut_arg("shell", |arg| arg.help(i18n.help_arg_shell())),
        _ => cmd, // No args to localize for other commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::i18n::Lang;
    use clap::CommandFactory;

    #[test]
    fn localize_command_shows_chinese_when_lang_zh() {
        let i18n = I18n::new(Lang::Zh);
        let cmd = Cli::command();
        let mut cmd = localize_command(cmd, &i18n);

        let help = cmd.render_help().to_string();
        assert!(help.contains("轻量级命令包装器"));
        assert!(help.contains("执行 shell 命令"));
    }

    #[test]
    fn localize_command_shows_english_when_lang_en() {
        let i18n = I18n::new(Lang::En);
        let cmd = Cli::command();
        let mut cmd = localize_command(cmd, &i18n);

        let help = cmd.render_help().to_string();
        assert!(help.contains("lightweight command wrapper"));
        assert!(help.contains("Execute a shell command"));
    }

    #[test]
    fn localize_command_localizes_global_args() {
        let i18n = I18n::new(Lang::Zh);
        let cmd = Cli::command();
        let mut cmd = localize_command(cmd, &i18n);

        let help = cmd.render_help().to_string();
        assert!(help.contains("这个任务做什么"));
        assert!(help.contains("为什么执行这个任务"));
        assert!(help.contains("消息语言"));
        assert!(help.contains("头信息输出流"));
    }

    #[test]
    fn localize_command_localizes_subcommands() {
        let i18n = I18n::new(Lang::Zh);
        let cmd = Cli::command();
        let cmd = localize_command(cmd, &i18n);

        // Check config subcommand
        let config_cmd = cmd.get_subcommands().find(|c| c.get_name() == "config");
        assert!(config_cmd.is_some());
        let mut config_cmd = config_cmd.unwrap().clone();
        let config_help = config_cmd.render_help().to_string();
        assert!(config_help.contains("获取配置值") && config_help.contains("设置配置值"));

        // Check init subcommand
        let init_cmd = cmd.get_subcommands().find(|c| c.get_name() == "init");
        assert!(init_cmd.is_some());
        let mut init_cmd = init_cmd.unwrap().clone();
        let init_help = init_cmd.render_help().to_string();
        assert!(init_help.contains("Claude Code") && init_help.contains("Codex"));
    }

    #[test]
    fn localize_command_localizes_script_args() {
        let i18n = I18n::new(Lang::Zh);
        let cmd = Cli::command();
        let cmd = localize_command(cmd, &i18n);

        let py_cmd = cmd.get_subcommands().find(|c| c.get_name() == "py");
        assert!(py_cmd.is_some());
        let mut py_cmd = py_cmd.unwrap().clone();
        let py_help = py_cmd.render_help().to_string();
        assert!(py_help.contains("内联脚本代码"));
        assert!(py_help.contains("脚本文件路径"));
    }

    #[test]
    fn get_command_about_returns_empty_for_unknown() {
        let i18n = I18n::new(Lang::En);
        let about = get_command_about("unknown_command", &i18n);
        assert!(about.is_empty());
    }

    #[test]
    fn localize_args_handles_unknown_command() {
        let i18n = I18n::new(Lang::Zh);
        let cmd = Command::new("unknown");
        let result = localize_args(cmd, "unknown", &i18n);
        // Should not panic and return the command unchanged
        assert_eq!(result.get_name(), "unknown");
    }
}
