use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::i18n::I18n;

#[derive(Parser, Debug)]
#[command(name = "shnote")]
#[command(version, about, long_about = None)]
#[command(subcommand_required = true)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// What this task does (required for run/py/node/pip/npm/npx, must appear before subcommand)
    #[arg(long, global = true)]
    pub what: Option<String>,

    /// Why this task is being executed (required for run/py/node/pip/npm/npx, must appear before subcommand)
    #[arg(long, global = true)]
    pub why: Option<String>,

    /// Language for messages (auto-detected by default)
    #[arg(long, global = true)]
    pub lang: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Execute a shell command (passthrough)
    Run(RunArgs),

    /// Execute a Python script
    Py(ScriptArgs),

    /// Execute a Node.js script
    Node(ScriptArgs),

    /// Execute pip (Python package manager)
    Pip(PassthroughArgs),

    /// Execute npm (Node.js package manager)
    Npm(PassthroughArgs),

    /// Execute npx (Node.js package runner)
    Npx(PassthroughArgs),

    /// Manage configuration
    Config(ConfigArgs),

    /// Initialize shnote rules for AI tools
    Init(InitArgs),

    /// Initialize environment (extract pueue binaries, etc.)
    Setup,

    /// Check environment dependencies (python/node/pueue)
    Doctor,

    /// Generate shell completion scripts
    Completions(CompletionsArgs),

    /// Show installation information
    Info,

    /// Update shnote to the latest version
    Update(UpdateArgs),

    /// Uninstall shnote
    Uninstall(UninstallArgs),
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Only check for updates, don't install
    #[arg(long)]
    pub check: bool,

    /// Force update even if already up to date
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct UninstallArgs {
    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,
}

impl Command {
    pub fn what_why_command_name(&self) -> Option<&'static str> {
        match self {
            Self::Run(_) => Some("run"),
            Self::Py(_) => Some("py"),
            Self::Node(_) => Some("node"),
            Self::Pip(_) => Some("pip"),
            Self::Npm(_) => Some("npm"),
            Self::Npx(_) => Some("npx"),
            Self::Config(_)
            | Self::Init(_)
            | Self::Setup
            | Self::Doctor
            | Self::Completions(_)
            | Self::Info
            | Self::Update(_)
            | Self::Uninstall(_) => None,
        }
    }

    pub fn requires_what_why(&self) -> bool {
        self.what_why_command_name().is_some()
    }
}

#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(ValueEnum, Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
    /// PowerShell
    #[value(name = "powershell")]
    PowerShell,
    /// Elvish shell
    Elvish,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Command and arguments to execute
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    pub command: Vec<OsString>,
}

#[derive(Args, Debug)]
pub struct PassthroughArgs {
    /// Arguments to pass through to the underlying command
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<OsString>,
}

#[derive(Args, Debug)]
pub struct ScriptArgs {
    /// Inline script code
    #[arg(short = 'c', long = "code", conflicts_with_all = ["file", "stdin"])]
    pub code: Option<String>,

    /// Script file path
    #[arg(short = 'f', long = "file", conflicts_with_all = ["code", "stdin"])]
    pub file: Option<PathBuf>,

    /// Read script from stdin (supports heredoc)
    #[arg(long = "stdin", conflicts_with_all = ["code", "file"])]
    pub stdin: bool,

    /// Arguments passed to the script
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<OsString>,
}

impl ScriptArgs {
    pub fn has_source(&self) -> bool {
        self.code.is_some() || self.file.is_some() || self.stdin
    }
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Get a configuration value
    Get {
        /// Configuration key (e.g., python, node, shell, language, output, color, what_color, why_color)
        key: String,
    },

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },

    /// List all configuration values
    List,

    /// Reset configuration to defaults
    Reset,

    /// Show configuration file path
    Path,
}

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Scope: user-level or project-level
    #[arg(short = 's', long = "scope", default_value = "user")]
    pub scope: Scope,

    #[command(subcommand)]
    pub target: InitTarget,
}

#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scope {
    /// User-level (writes to ~/.claude, ~/.codex, ~/.gemini)
    #[default]
    #[value(alias = "u")]
    User,
    /// Project-level (writes to .claude, .codex, .gemini in current directory)
    #[value(alias = "p")]
    Project,
}

#[derive(Subcommand, Debug)]
pub enum InitTarget {
    /// Install shnote rules for Claude Code (>= 2.0.64: ~/.claude/rules/shnote.md; otherwise: ~/.claude/CLAUDE.md)
    Claude,

    /// Install or update shnote rules for Codex (~/.codex/AGENTS.md)
    Codex,

    /// Install or update shnote rules for Gemini (~/.gemini/GEMINI.md)
    Gemini,
}

pub fn validate_what_why(i18n: &I18n, cli: &Cli) -> anyhow::Result<()> {
    if let Some(cmd_name) = cli.command.what_why_command_name() {
        if cli.what.is_none() || cli.why.is_none() {
            anyhow::bail!("{}", i18n.err_missing_what_why(cmd_name));
        }
    } else if cli.what.is_some() || cli.why.is_some() {
        anyhow::bail!("{}", i18n.err_reject_root_meta());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    fn test_i18n() -> I18n {
        I18n::new(Lang::En)
    }

    #[test]
    fn command_requires_what_why() {
        use std::ffi::OsString;

        let run_cmd = Command::Run(RunArgs {
            command: vec![OsString::from("ls")],
        });
        assert!(run_cmd.requires_what_why());

        let py_cmd = Command::Py(ScriptArgs {
            code: Some("print('hello')".to_string()),
            file: None,
            stdin: false,
            args: vec![],
        });
        assert!(py_cmd.requires_what_why());

        let node_cmd = Command::Node(ScriptArgs {
            code: Some("console.log('hello')".to_string()),
            file: None,
            stdin: false,
            args: vec![],
        });
        assert!(node_cmd.requires_what_why());

        let config_cmd = Command::Config(ConfigArgs {
            action: ConfigAction::List,
        });
        assert!(!config_cmd.requires_what_why());

        let setup_cmd = Command::Setup;
        assert!(!setup_cmd.requires_what_why());

        let doctor_cmd = Command::Doctor;
        assert!(!doctor_cmd.requires_what_why());

        let completions_cmd = Command::Completions(CompletionsArgs { shell: Shell::Bash });
        assert!(!completions_cmd.requires_what_why());
    }

    #[test]
    fn script_args_has_source() {
        let with_code = ScriptArgs {
            code: Some("print('hello')".to_string()),
            file: None,
            stdin: false,
            args: vec![],
        };
        assert!(with_code.has_source());

        let with_file = ScriptArgs {
            code: None,
            file: Some(std::path::PathBuf::from("test.py")),
            stdin: false,
            args: vec![],
        };
        assert!(with_file.has_source());

        let with_stdin = ScriptArgs {
            code: None,
            file: None,
            stdin: true,
            args: vec![],
        };
        assert!(with_stdin.has_source());

        let no_source = ScriptArgs {
            code: None,
            file: None,
            stdin: false,
            args: vec![],
        };
        assert!(!no_source.has_source());
    }

    #[test]
    fn validate_what_why_missing() {
        use std::ffi::OsString;

        let i18n = test_i18n();
        let cli = Cli {
            what: None,
            why: None,
            lang: None,
            command: Command::Run(RunArgs {
                command: vec![OsString::from("ls")],
            }),
        };
        assert!(validate_what_why(&i18n, &cli).is_err());
    }

    #[test]
    fn validate_what_why_present() {
        use std::ffi::OsString;

        let i18n = test_i18n();
        let cli = Cli {
            what: Some("test".to_string()),
            why: Some("testing".to_string()),
            lang: None,
            command: Command::Run(RunArgs {
                command: vec![OsString::from("ls")],
            }),
        };
        assert!(validate_what_why(&i18n, &cli).is_ok());
    }

    #[test]
    fn validate_what_why_rejected_for_non_exec() {
        let i18n = test_i18n();
        let cli = Cli {
            what: Some("test".to_string()),
            why: Some("testing".to_string()),
            lang: None,
            command: Command::Doctor,
        };
        assert!(validate_what_why(&i18n, &cli).is_err());
    }
}
