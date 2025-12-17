mod cli;
mod config;
mod doctor;
mod executor;
mod i18n;
mod info;
mod init;
mod localize;
mod pueue_embed;
mod shell;
mod uninstall;
mod update;
#[cfg(test)]
mod test_support;

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches};
use clap_complete::{generate, Shell as CompletionShell};

use crate::cli::{Cli, Command, ConfigAction, Shell};
use crate::config::Config;
use crate::i18n::I18n;

fn main() -> ExitCode {
    // 1. Pre-parse to extract --lang argument (if any)
    let pre_args: Vec<String> = std::env::args().collect();
    let lang_override = extract_lang_arg(&pre_args);

    // 2. Load config (ignore errors, use defaults)
    let config = Config::load().unwrap_or_default();

    // 3. Detect language
    let lang = i18n::detect_lang(lang_override.as_deref(), &config.i18n.language);
    let i18n = I18n::new(lang);

    // 4. Get and localize Command
    let cmd = Cli::command();
    let cmd = localize::localize_command(cmd, &i18n);

    // 5. Parse arguments with localized command
    // Note: get_matches() handles all parsing errors (exits on failure),
    // so from_arg_matches cannot fail with a valid ArgMatches.
    let cli = Cli::from_arg_matches(&cmd.get_matches())
        .expect("clap derive should match parsed arguments");

    // Validate --what/--why
    if let Err(e) = cli::validate_what_why(&i18n, &cli) {
        eprintln!("error: {e}");
        return ExitCode::from(1);
    }

    // Output WHAT/WHY for execution commands (if not in quiet mode)
    if cli.command.requires_what_why() && config.should_print_header() {
        // Safe: `validate_what_why` above guarantees these are present for execution commands.
        let what = cli.what.as_deref().expect("validated --what");
        let why = cli.why.as_deref().expect("validated --why");
        println!("WHAT: {what}");
        println!("WHY:  {why}");
        // Flush stdout to ensure WHAT/WHY appears before command output
        let _ = io::stdout().flush();
    }

    // Dispatch command
    match run(&i18n, &config, cli.command) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:?}");
            ExitCode::from(1)
        }
    }
}

fn run(i18n: &I18n, config: &Config, command: Command) -> Result<ExitCode> {
    match command {
        Command::Run(args) => executor::exec_run(i18n, config, args),

        Command::Py(args) => executor::exec_py(i18n, config, args),

        Command::Node(args) => executor::exec_node(i18n, config, args),

        Command::Pip(args) => executor::exec_pip(i18n, config, args),

        Command::Npm(args) => executor::exec_npm(i18n, config, args),

        Command::Npx(args) => executor::exec_npx(i18n, config, args),

        Command::Config(args) => {
            handle_config(i18n, args)?;
            Ok(ExitCode::SUCCESS)
        }

        Command::Init(args) => {
            init::run_init(i18n, args.target, args.scope)?;
            Ok(ExitCode::SUCCESS)
        }

        Command::Setup => {
            pueue_embed::run_setup(i18n)?;
            Ok(ExitCode::SUCCESS)
        }

        Command::Doctor => {
            let results = doctor::run_doctor(i18n, config);
            doctor::print_doctor_results(i18n, &results);
            let all_ok = results.iter().all(|r| r.ok);
            Ok(if all_ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }

        Command::Completions(args) => {
            generate_completions(args.shell);
            Ok(ExitCode::SUCCESS)
        }

        Command::Info => {
            info::run_info(i18n)?;
            Ok(ExitCode::SUCCESS)
        }

        Command::Update(args) => {
            update::run_update(i18n, args)?;
            Ok(ExitCode::SUCCESS)
        }

        Command::Uninstall(args) => {
            uninstall::run_uninstall(i18n, args)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let shell = match shell {
        Shell::Bash => CompletionShell::Bash,
        Shell::Zsh => CompletionShell::Zsh,
        Shell::Fish => CompletionShell::Fish,
        Shell::PowerShell => CompletionShell::PowerShell,
        Shell::Elvish => CompletionShell::Elvish,
    };
    generate(shell, &mut cmd, "shnote", &mut io::stdout());
}

fn handle_config(i18n: &I18n, args: cli::ConfigArgs) -> Result<()> {
    match args.action {
        ConfigAction::Get { key } => {
            let config = Config::load()?;
            match config.get(&key) {
                Some(value) => println!("{value}"),
                None => {
                    anyhow::bail!("{}", i18n.config_key_not_found(&key));
                }
            }
        }

        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            if config.set(i18n, &key, &value)? {
                config.save(i18n)?;
                println!("{}", i18n.config_updated(&key, &value));
            } else {
                anyhow::bail!("{}", i18n.config_key_not_found(&key));
            }
        }

        ConfigAction::List => {
            let config = Config::load()?;
            for (key, value) in config.list() {
                println!("{key} = {value}");
            }
        }

        ConfigAction::Reset => {
            Config::reset(i18n)?;
            println!("{}", i18n.config_reset_done());
        }

        ConfigAction::Path => {
            let path = config::config_path()?;
            println!("{}", path.display());
        }
    }

    Ok(())
}

/// Extract --lang argument from command line args before full parsing.
///
/// This is needed because we need to know the language before parsing to
/// localize the help text. The --lang argument can appear anywhere in the
/// command line as a global argument.
fn extract_lang_arg(args: &[String]) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--lang" {
            return args.get(i + 1).cloned();
        }
        if let Some(lang) = arg.strip_prefix("--lang=") {
            return Some(lang.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::test_support::{env_lock, EnvVarGuard};
    use std::ffi::OsString;
    use std::fs;
    use tempfile::TempDir;

    #[cfg(unix)]
    use crate::test_support::write_executable;

    #[test]
    fn generate_completions_all_shells_does_not_panic() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ] {
            generate_completions(shell);
        }
    }

    #[test]
    fn handle_config_success_paths() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = I18n::new(Lang::En);

        handle_config(
            &i18n,
            cli::ConfigArgs {
                action: ConfigAction::Path,
            },
        )
        .unwrap();

        handle_config(
            &i18n,
            cli::ConfigArgs {
                action: ConfigAction::Set {
                    key: "python".to_string(),
                    value: "/bin/sh".to_string(),
                },
            },
        )
        .unwrap();

        handle_config(
            &i18n,
            cli::ConfigArgs {
                action: ConfigAction::Get {
                    key: "python".to_string(),
                },
            },
        )
        .unwrap();

        let err = handle_config(
            &i18n,
            cli::ConfigArgs {
                action: ConfigAction::Get {
                    key: "unknown_key".to_string(),
                },
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown"));

        handle_config(
            &i18n,
            cli::ConfigArgs {
                action: ConfigAction::List,
            },
        )
        .unwrap();

        handle_config(
            &i18n,
            cli::ConfigArgs {
                action: ConfigAction::Reset,
            },
        )
        .unwrap();
    }

    #[test]
    fn handle_config_set_propagates_error_when_value_invalid() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = I18n::new(Lang::En);
        let args = cli::ConfigArgs {
            action: ConfigAction::Set {
                key: "shell".to_string(),
                value: "invalid".to_string(),
            },
        };

        let err = handle_config(&i18n, args).unwrap_err();
        assert!(err.to_string().contains("invalid"));
    }

    #[cfg(unix)]
    #[test]
    fn run_covers_all_command_variants_in_unit_tests() {
        use std::os::unix::fs::PermissionsExt;

        let _lock = env_lock();
        let home_dir = TempDir::new().unwrap();
        let tools_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");
        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        let bash = tools_dir.path().join("bash");
        write_executable(&bash, "#!/bin/sh\necho \"bash 1.0\"\nexit 0\n").unwrap();
        let _shell_guard = EnvVarGuard::set("SHELL", &bash);

        let python = tools_dir.path().join("python3");
        write_executable(&python, "#!/bin/sh\necho \"Python 3.0\"\nexit 0\n").unwrap();

        let node = tools_dir.path().join("node");
        write_executable(&node, "#!/bin/sh\necho \"v1.0\"\nexit 0\n").unwrap();

        let npm = tools_dir.path().join("npm");
        write_executable(&npm, "#!/bin/sh\nexit 0\n").unwrap();

        let npx = tools_dir.path().join("npx");
        write_executable(&npx, "#!/bin/sh\nexit 0\n").unwrap();

        let dummy = tools_dir.path().join("dummy");
        write_executable(&dummy, "#!/bin/sh\nexit 0\n").unwrap();

        let bin_dir = home_dir.path().join(".shnote/bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::set_permissions(&bin_dir, fs::Permissions::from_mode(0o755)).unwrap();

        let pueue = bin_dir.join(crate::config::pueue_binary_name());
        write_executable(&pueue, "#!/bin/sh\necho \"pueue 4.0\"\nexit 0\n").unwrap();

        let pueued = bin_dir.join(crate::config::pueued_binary_name());
        write_executable(&pueued, "#!/bin/sh\necho \"pueued 4.0\"\nexit 0\n").unwrap();

        let i18n = I18n::new(Lang::En);
        let mut config = Config::default();
        config.paths.python = python.display().to_string();
        config.paths.node = node.display().to_string();
        config.paths.shell = "auto".to_string();

        let code = run(
            &i18n,
            &config,
            Command::Run(cli::RunArgs {
                command: vec![OsString::from("dummy")],
            }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let code = run(
            &i18n,
            &config,
            Command::Py(cli::ScriptArgs {
                code: Some("print(1)".to_string()),
                file: None,
                stdin: false,
                args: vec![],
            }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let code = run(
            &i18n,
            &config,
            Command::Node(cli::ScriptArgs {
                code: Some("console.log(1)".to_string()),
                file: None,
                stdin: false,
                args: vec![],
            }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let code = run(
            &i18n,
            &config,
            Command::Pip(cli::PassthroughArgs {
                args: vec![OsString::from("--version")],
            }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let code = run(
            &i18n,
            &config,
            Command::Npm(cli::PassthroughArgs {
                args: vec![OsString::from("--version")],
            }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let code = run(
            &i18n,
            &config,
            Command::Npx(cli::PassthroughArgs {
                args: vec![OsString::from("--version")],
            }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let code = run(
            &i18n,
            &config,
            Command::Config(cli::ConfigArgs {
                action: ConfigAction::List,
            }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let code = run(
            &i18n,
            &config,
            Command::Completions(cli::CompletionsArgs { shell: Shell::Bash }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let code = run(
            &i18n,
            &config,
            Command::Init(cli::InitArgs {
                scope: cli::Scope::User,
                target: cli::InitTarget::Claude,
            }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let code = run(&i18n, &config, Command::Doctor).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn run_config_propagates_error_when_handle_config_fails() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = I18n::new(Lang::En);
        let config = Config::default();
        let cmd = Command::Config(cli::ConfigArgs {
            action: ConfigAction::Path,
        });

        let err = run(&i18n, &config, cmd).unwrap_err();
        assert!(err
            .to_string()
            .contains("failed to determine home directory"));
    }

    #[test]
    fn handle_config_set_unknown_key_errors() {
        let _lock = env_lock();
        let home_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());

        let i18n = I18n::new(Lang::En);
        let args = cli::ConfigArgs {
            action: ConfigAction::Set {
                key: "unknown_key".to_string(),
                value: "value".to_string(),
            },
        };

        let err = handle_config(&i18n, args).unwrap_err();
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn handle_config_get_errors_when_config_load_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();
        fs::write(temp_dir.path().join(".shnote/config.toml"), "not = [valid").unwrap();

        let i18n = I18n::new(Lang::En);
        let args = cli::ConfigArgs {
            action: ConfigAction::Get {
                key: "python".to_string(),
            },
        };

        let err = handle_config(&i18n, args).unwrap_err();
        assert!(err.to_string().contains("failed to parse config file"));
    }

    #[test]
    fn handle_config_set_errors_when_config_load_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();
        fs::write(temp_dir.path().join(".shnote/config.toml"), "not = [valid").unwrap();

        let i18n = I18n::new(Lang::En);
        let args = cli::ConfigArgs {
            action: ConfigAction::Set {
                key: "python".to_string(),
                value: "/bin/sh".to_string(),
            },
        };

        let err = handle_config(&i18n, args).unwrap_err();
        assert!(err.to_string().contains("failed to parse config file"));
    }

    #[test]
    fn handle_config_list_errors_when_config_load_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();
        fs::write(temp_dir.path().join(".shnote/config.toml"), "not = [valid").unwrap();

        let i18n = I18n::new(Lang::En);
        let args = cli::ConfigArgs {
            action: ConfigAction::List,
        };

        let err = handle_config(&i18n, args).unwrap_err();
        assert!(err.to_string().contains("failed to parse config file"));
    }

    #[cfg(unix)]
    #[test]
    fn handle_config_set_errors_when_config_save_fails() {
        use std::os::unix::fs::PermissionsExt;

        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        fs::create_dir_all(temp_dir.path().join(".shnote")).unwrap();

        // Valid config that can be read, but make it read-only so save fails.
        let config_path = temp_dir.path().join(".shnote/config.toml");
        fs::write(
            &config_path,
            toml::to_string_pretty(&Config::default()).unwrap(),
        )
        .unwrap();
        fs::set_permissions(&config_path, fs::Permissions::from_mode(0o444)).unwrap();

        let i18n = I18n::new(Lang::En);
        let args = cli::ConfigArgs {
            action: ConfigAction::Set {
                key: "python".to_string(),
                value: "/bin/sh".to_string(),
            },
        };

        let err = handle_config(&i18n, args).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_write_config(&config_path.display().to_string())));
    }

    #[test]
    fn handle_config_reset_errors_when_save_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Make ~/.shnote a file so ~/.shnote/config.toml cannot be created.
        fs::write(temp_dir.path().join(".shnote"), "not a dir").unwrap();

        let i18n = I18n::new(Lang::En);
        let args = cli::ConfigArgs {
            action: ConfigAction::Reset,
        };

        let err = handle_config(&i18n, args).unwrap_err();
        assert!(err.to_string().contains(
            &i18n.err_create_config_dir(&temp_dir.path().join(".shnote").display().to_string())
        ));
    }

    #[test]
    fn handle_config_path_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = I18n::new(Lang::En);
        let args = cli::ConfigArgs {
            action: ConfigAction::Path,
        };

        let err = handle_config(&i18n, args).unwrap_err();
        assert!(err
            .to_string()
            .contains("failed to determine home directory"));
    }

    #[test]
    fn run_init_propagates_error() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = I18n::new(Lang::En);
        let config = Config::default();
        let cmd = Command::Init(cli::InitArgs {
            scope: cli::Scope::User,
            target: cli::InitTarget::Claude,
        });

        let err = run(&i18n, &config, cmd).unwrap_err();
        assert!(err.to_string().contains(i18n.err_home_dir()));
    }

    #[cfg(unix)]
    #[test]
    fn run_setup_propagates_error() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Make ~/.shnote a file so setup fails when creating ~/.shnote/bin.
        fs::write(temp_dir.path().join(".shnote"), "not a dir").unwrap();

        let i18n = I18n::new(Lang::En);
        let config = Config::default();
        let err = run(&i18n, &config, Command::Setup).unwrap_err();
        assert!(err.to_string().contains("failed"));
    }

    #[cfg(unix)]
    #[test]
    fn run_doctor_returns_failure_exit_code_when_any_check_fails() {
        let _lock = env_lock();
        let home_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());

        let path_dir = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());
        let _shell_guard = EnvVarGuard::set("SHELL", "/nonexistent/bash");

        let i18n = I18n::new(Lang::En);
        let mut config = Config::default();
        config.paths.python = "/nonexistent/python".to_string();
        config.paths.node = "/nonexistent/node".to_string();
        config.paths.shell = "bash".to_string();

        let code = run(&i18n, &config, Command::Doctor).unwrap();
        assert_eq!(code, ExitCode::from(1));
    }

    #[cfg(unix)]
    #[test]
    fn run_setup_succeeds_with_fake_curl_and_shasum() {
        let _lock = env_lock();
        let home_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path());

        let tools_dir = TempDir::new().unwrap();

        let curl = tools_dir.path().join("curl");
        write_executable(
            &curl,
            "#!/bin/sh\n\
dest=\"\"\n\
while [ \"$#\" -gt 0 ]; do\n\
  if [ \"$1\" = \"-o\" ]; then\n\
    dest=\"$2\"\n\
    break\n\
  fi\n\
  shift\n\
done\n\
if [ -z \"$dest\" ]; then\n\
  exit 2\n\
fi\n\
echo \"dummy\" > \"$dest\"\n\
exit 0\n",
        )
        .unwrap();

        let shasum = tools_dir.path().join("shasum");
        let pueue_hash = crate::pueue_embed::checksums::PUEUE_SHA256;
        let pueued_hash = crate::pueue_embed::checksums::PUEUED_SHA256;
        let shasum_script = format!(
            "#!/bin/sh\n\
file=\"$3\"\n\
case \"$file\" in\n\
  *pueue) echo \"{pueue_hash}  $file\" ;;\n\
  *pueued) echo \"{pueued_hash}  $file\" ;;\n\
  *) echo \"{pueue_hash}  $file\" ;;\n\
esac\n\
exit 0\n"
        );
        write_executable(&shasum, &shasum_script).unwrap();

        let _path_guard = EnvVarGuard::set("PATH", tools_dir.path());

        let i18n = I18n::new(Lang::En);
        let config = Config::default();
        let code = run(&i18n, &config, Command::Setup).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let bin_dir = crate::config::shnote_bin_dir().unwrap();
        assert!(bin_dir.join(crate::config::pueue_binary_name()).exists());
        assert!(bin_dir.join(crate::config::pueued_binary_name()).exists());
    }

    #[test]
    fn extract_lang_arg_with_equals_syntax() {
        let args = vec![
            "shnote".to_string(),
            "--lang=zh".to_string(),
            "doctor".to_string(),
        ];
        assert_eq!(extract_lang_arg(&args), Some("zh".to_string()));
    }

    #[test]
    fn extract_lang_arg_with_space_syntax() {
        let args = vec![
            "shnote".to_string(),
            "--lang".to_string(),
            "en".to_string(),
            "doctor".to_string(),
        ];
        assert_eq!(extract_lang_arg(&args), Some("en".to_string()));
    }

    #[test]
    fn extract_lang_arg_not_present() {
        let args = vec!["shnote".to_string(), "doctor".to_string()];
        assert_eq!(extract_lang_arg(&args), None);
    }

    #[test]
    fn extract_lang_arg_at_end_without_value() {
        let args = vec!["shnote".to_string(), "--lang".to_string()];
        assert_eq!(extract_lang_arg(&args), None);
    }
}
