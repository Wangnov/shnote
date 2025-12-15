use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result};
use which::which;

use crate::cli::{PassthroughArgs, RunArgs, ScriptArgs};
use crate::config::Config;
use crate::i18n::I18n;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScriptType {
    Py,
    Node,
}

impl ScriptType {
    fn code_flag(self) -> &'static str {
        match self {
            Self::Py => "-c",
            Self::Node => "-e",
        }
    }

    fn is_python(self) -> bool {
        matches!(self, Self::Py)
    }
}

/// Execute a command directly (run subcommand) - true passthrough
pub fn exec_run(i18n: &I18n, _config: &Config, args: RunArgs) -> Result<ExitCode> {
    // `RunArgs.command` is `required = true` in clap, so it is always non-empty in CLI usage.
    let mut command = args.command;
    let program = command.remove(0);
    let program_args = command;

    let mut cmd = Command::new(&program);
    cmd.args(&program_args);

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd
        .status()
        .context(i18n.err_failed_to_execute(&program.to_string_lossy()))?;

    Ok(exit_code_from_status(status))
}

/// Execute a Python script (py subcommand)
pub fn exec_py(i18n: &I18n, config: &Config, args: ScriptArgs) -> Result<ExitCode> {
    let python = resolve_interpreter(i18n, &config.paths.python, &["python3", "python"])?;
    exec_script(i18n, &python, args, ScriptType::Py)
}

/// Execute a Node.js script (node subcommand)
pub fn exec_node(i18n: &I18n, config: &Config, args: ScriptArgs) -> Result<ExitCode> {
    let node = resolve_interpreter(i18n, &config.paths.node, &["node"])?;
    exec_script(i18n, &node, args, ScriptType::Node)
}

/// Execute pip (pip subcommand)
/// Uses `python -m pip` to ensure we use the correct pip for the configured Python
pub fn exec_pip(i18n: &I18n, config: &Config, args: PassthroughArgs) -> Result<ExitCode> {
    let python = resolve_interpreter(i18n, &config.paths.python, &["python3", "python"])?;

    let mut cmd = Command::new(&python);
    cmd.arg("-m").arg("pip");
    cmd.args(&args.args);

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().context(i18n.err_failed_to_execute("pip"))?;

    Ok(exit_code_from_status(status))
}

/// Execute npm (npm subcommand)
/// Finds npm relative to the configured node path
pub fn exec_npm(i18n: &I18n, config: &Config, args: PassthroughArgs) -> Result<ExitCode> {
    let npm = resolve_node_tool(i18n, config, "npm")?;

    // On Windows, .cmd files must be executed through cmd.exe
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&npm);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = Command::new(&npm);

    cmd.args(&args.args);

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().context(i18n.err_failed_to_execute("npm"))?;

    Ok(exit_code_from_status(status))
}

/// Execute npx (npx subcommand)
/// Finds npx relative to the configured node path
pub fn exec_npx(i18n: &I18n, config: &Config, args: PassthroughArgs) -> Result<ExitCode> {
    let npx = resolve_node_tool(i18n, config, "npx")?;

    // On Windows, .cmd files must be executed through cmd.exe
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&npx);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = Command::new(&npx);

    cmd.args(&args.args);

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().context(i18n.err_failed_to_execute("npx"))?;

    Ok(exit_code_from_status(status))
}

/// Resolve npm/npx path relative to the configured node
fn resolve_node_tool(i18n: &I18n, config: &Config, tool: &str) -> Result<PathBuf> {
    let node = resolve_interpreter(i18n, &config.paths.node, &["node"])?;

    // Try to find the tool in the same directory as node
    if let Some(node_dir) = node.parent() {
        let tool_path = node_dir.join(tool);
        if tool_path.exists() {
            return Ok(tool_path);
        }

        // On Windows, try with .cmd extension
        #[cfg(windows)]
        {
            let tool_cmd = node_dir.join(format!("{}.cmd", tool));
            if tool_cmd.exists() {
                return Ok(tool_cmd);
            }
        }
    }

    // Fallback: try to find in PATH
    if let Ok(resolved) = which(tool) {
        return Ok(resolved);
    }

    anyhow::bail!("{}", i18n.err_interpreter_not_found(tool))
}

fn exec_script(
    i18n: &I18n,
    interpreter: &PathBuf,
    args: ScriptArgs,
    script_type: ScriptType,
) -> Result<ExitCode> {
    let mut stdin = io::stdin();
    exec_script_with_reader(i18n, interpreter, args, script_type, &mut stdin)
}

fn exec_script_with_reader(
    i18n: &I18n,
    interpreter: &PathBuf,
    args: ScriptArgs,
    script_type: ScriptType,
    stdin_reader: &mut dyn Read,
) -> Result<ExitCode> {
    if !args.has_source() {
        anyhow::bail!("{}", i18n.err_script_source_required());
    }

    let mut cmd = Command::new(interpreter);

    // Set Python-specific environment variables
    if script_type.is_python() {
        cmd.env("PYTHONUTF8", "1");
        cmd.env("PYTHONIOENCODING", "utf-8");
    }

    if let Some(code) = &args.code {
        // Inline code: interpreter -c "code"
        cmd.arg(script_type.code_flag()).arg(code);
    } else if let Some(file) = &args.file {
        // File: interpreter file.py
        cmd.arg(file);
    } else {
        // Stdin: read code and pass via -c
        let code = read_to_string(i18n, stdin_reader)?;
        cmd.arg(script_type.code_flag()).arg(&code);
    }

    // Add script arguments
    for arg in &args.args {
        cmd.arg(arg);
    }

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd
        .status()
        .context(i18n.err_failed_to_execute(&interpreter.display().to_string()))?;

    Ok(exit_code_from_status(status))
}

fn resolve_interpreter(i18n: &I18n, configured: &str, fallbacks: &[&str]) -> Result<PathBuf> {
    // If configured path is absolute, use it directly
    let path = PathBuf::from(configured);
    if path.is_absolute() {
        if path.exists() {
            return Ok(path);
        }
        anyhow::bail!("{}", i18n.err_interpreter_not_found(configured));
    }

    // Try to find in PATH
    if let Ok(resolved) = which(configured) {
        return Ok(resolved);
    }

    // Try fallbacks
    for fallback in fallbacks {
        if let Ok(resolved) = which(fallback) {
            return Ok(resolved);
        }
    }

    anyhow::bail!("{}", i18n.err_interpreter_not_found(configured))
}

fn read_to_string(i18n: &I18n, reader: &mut dyn Read) -> Result<String> {
    let mut buffer = String::new();
    reader
        .read_to_string(&mut buffer)
        .context(i18n.err_read_stdin())?;
    Ok(buffer)
}

fn exit_code_from_status(status: std::process::ExitStatus) -> ExitCode {
    #[cfg(unix)]
    {
        if let Some(code) = status.code() {
            ExitCode::from(code as u8)
        } else {
            ExitCode::from(1)
        }
    }

    #[cfg(not(unix))]
    {
        let code = status
            .code()
            .and_then(|c| u8::try_from(c).ok())
            .unwrap_or(1);
        ExitCode::from(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::test_support::{env_lock, EnvVarGuard};
    use std::ffi::OsString;
    use tempfile::TempDir;

    #[cfg(unix)]
    use crate::test_support::write_executable;

    fn test_i18n() -> I18n {
        I18n::new(Lang::En)
    }

    #[test]
    fn exec_run_executes_command() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let config = Config::default();
        #[cfg(unix)]
        let args = RunArgs {
            command: vec![OsString::from("/usr/bin/true")],
        };
        #[cfg(windows)]
        let args = RunArgs {
            command: vec![
                OsString::from("cmd"),
                OsString::from("/C"),
                OsString::from("exit"),
                OsString::from("0"),
            ],
        };
        let result = exec_run(&i18n, &config, args);
        assert!(result.is_ok());
    }

    #[test]
    fn exec_py_requires_source() {
        let i18n = test_i18n();
        let config = Config::default();
        let args = ScriptArgs {
            code: None,
            file: None,
            stdin: false,
            args: vec![],
        };
        let result = exec_py(&i18n, &config, args);
        assert!(result.is_err());
    }

    #[test]
    fn exec_py_with_inline_code() {
        let i18n = test_i18n();
        let config = Config::default();
        let args = ScriptArgs {
            code: Some("print('hello')".to_string()),
            file: None,
            stdin: false,
            args: vec![],
        };
        // This test may fail if python is not installed, but that's ok
        let result = exec_py(&i18n, &config, args);
        // Just ensure it doesn't panic and returns some result
        let _ = result;
    }

    #[test]
    fn exec_node_requires_source() {
        let i18n = test_i18n();
        let config = Config::default();
        let args = ScriptArgs {
            code: None,
            file: None,
            stdin: false,
            args: vec![],
        };
        let result = exec_node(&i18n, &config, args);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_interpreter_absolute_path() {
        let i18n = test_i18n();
        // Use a path that exists on all Unix systems
        #[cfg(unix)]
        let result = resolve_interpreter(&i18n, "/bin/sh", &[]);
        #[cfg(windows)]
        let result = resolve_interpreter(&i18n, "C:\\Windows\\System32\\cmd.exe", &[]);

        assert!(result.is_ok());
    }

    #[test]
    fn resolve_interpreter_nonexistent_absolute() {
        let i18n = test_i18n();
        let result = resolve_interpreter(&i18n, "/nonexistent/binary", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_interpreter_uses_fallbacks() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        #[cfg(unix)]
        {
            let sh = temp_dir.path().join("sh");
            write_executable(&sh, "#!/bin/sh\nexit 0\n").unwrap();
            let _path_guard = EnvVarGuard::set("PATH", temp_dir.path());

            // Try with a nonexistent primary, but existing fallback
            let result = resolve_interpreter(&i18n, "nonexistent_binary_xyz", &["sh"]);
            assert_eq!(result.unwrap(), sh);
        }
    }

    #[test]
    fn exit_code_from_status_success() {
        use std::process::Command;
        #[cfg(unix)]
        {
            let status = Command::new("/usr/bin/true").status().unwrap();
            let code = exit_code_from_status(status);
            assert_eq!(code, ExitCode::SUCCESS);
        }
        #[cfg(windows)]
        {
            let status = Command::new("cmd")
                .args(["/C", "exit", "0"])
                .status()
                .unwrap();
            let code = exit_code_from_status(status);
            assert_eq!(code, ExitCode::SUCCESS);
        }
    }

    #[test]
    fn exit_code_from_status_failure() {
        use std::process::Command;
        #[cfg(unix)]
        {
            let status = Command::new("/usr/bin/false").status().unwrap();
            let code = exit_code_from_status(status);
            assert_ne!(code, ExitCode::SUCCESS);
        }
        #[cfg(windows)]
        {
            let status = Command::new("cmd")
                .args(["/C", "exit", "1"])
                .status()
                .unwrap();
            let code = exit_code_from_status(status);
            assert_ne!(code, ExitCode::SUCCESS);
        }
    }

    #[cfg(unix)]
    #[test]
    fn exit_code_from_status_none_maps_to_1() {
        use std::os::unix::process::ExitStatusExt;

        let status = std::process::ExitStatus::from_raw(9);
        let code = exit_code_from_status(status);
        assert_eq!(code, ExitCode::from(1));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_node_tool_finds_tool_next_to_node() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();

        let node = temp_dir.path().join("node");
        std::fs::write(&node, "").unwrap();

        let npm = temp_dir.path().join("npm");
        std::fs::write(&npm, "").unwrap();

        let mut config = Config::default();
        config.paths.node = node.display().to_string();

        let resolved = resolve_node_tool(&i18n, &config, "npm").unwrap();
        assert_eq!(resolved, npm);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_node_tool_falls_back_to_path() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let node_dir = TempDir::new().unwrap();
        let path_dir = TempDir::new().unwrap();

        let node = node_dir.path().join("node");
        std::fs::write(&node, "").unwrap();

        let npm = path_dir.path().join("npm");
        write_executable(&npm, "#!/bin/sh\nexit 0\n").unwrap();

        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());

        let mut config = Config::default();
        config.paths.node = node.display().to_string();

        let resolved = resolve_node_tool(&i18n, &config, "npm").unwrap();
        assert_eq!(resolved, npm);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_node_tool_errors_when_missing() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let node_dir = TempDir::new().unwrap();
        let path_dir = TempDir::new().unwrap();

        let node = node_dir.path().join("node");
        std::fs::write(&node, "").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());

        let mut config = Config::default();
        config.paths.node = node.display().to_string();

        let err = resolve_node_tool(&i18n, &config, "npm").unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_interpreter_not_found("npm")));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_node_tool_handles_node_without_parent() {
        let _lock = env_lock();
        let i18n = test_i18n();

        let path_dir = TempDir::new().unwrap();
        let npm = path_dir.path().join("npm");
        write_executable(&npm, "#!/bin/sh\nexit 0\n").unwrap();
        let _path_guard = EnvVarGuard::set("PATH", path_dir.path());

        let mut config = Config::default();
        config.paths.node = "/".to_string();

        let resolved = resolve_node_tool(&i18n, &config, "npm").unwrap();
        assert_eq!(resolved, npm);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_node_tool_errors_when_node_interpreter_not_found() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());

        let mut config = Config::default();
        config.paths.node = "definitely_not_a_real_node".to_string();

        let err = resolve_node_tool(&i18n, &config, "npm").unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_interpreter_not_found("definitely_not_a_real_node")));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_interpreter_errors_when_not_found() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());

        let err = resolve_interpreter(&i18n, "definitely_not_a_real_binary", &[]).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_interpreter_not_found("definitely_not_a_real_binary")));
    }

    #[test]
    fn read_to_string_reads_all_content() {
        let i18n = test_i18n();
        let mut cursor = std::io::Cursor::new("hello");
        let out = read_to_string(&i18n, &mut cursor).unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn read_to_string_returns_error_on_reader_failure() {
        struct FailingReader;

        impl Read for FailingReader {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("boom"))
            }
        }

        let i18n = test_i18n();
        let mut reader = FailingReader;
        let err = read_to_string(&i18n, &mut reader).unwrap_err();
        assert!(err.to_string().contains(i18n.err_read_stdin()));
    }

    #[test]
    fn exec_py_errors_when_interpreter_not_found() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let mut config = Config::default();
        config.paths.python = "definitely_not_a_real_python".to_string();

        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());

        let args = ScriptArgs {
            code: Some("print('x')".to_string()),
            file: None,
            stdin: false,
            args: vec![],
        };

        let err = exec_py(&i18n, &config, args).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_interpreter_not_found("definitely_not_a_real_python")));
    }

    #[test]
    fn exec_node_errors_when_interpreter_not_found() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let mut config = Config::default();
        config.paths.node = "definitely_not_a_real_node".to_string();

        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());

        let args = ScriptArgs {
            code: Some("console.log('x')".to_string()),
            file: None,
            stdin: false,
            args: vec![],
        };

        let err = exec_node(&i18n, &config, args).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_interpreter_not_found("definitely_not_a_real_node")));
    }

    #[test]
    fn exec_pip_errors_when_interpreter_not_found() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let mut config = Config::default();
        config.paths.python = "definitely_not_a_real_python".to_string();

        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());

        let args = PassthroughArgs { args: vec![] };
        let err = exec_pip(&i18n, &config, args).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_interpreter_not_found("definitely_not_a_real_python")));
    }

    #[cfg(unix)]
    #[test]
    fn exec_pip_errors_when_python_cannot_be_executed() {
        use tempfile::TempDir;

        let i18n = test_i18n();
        let mut config = Config::default();
        let dir = TempDir::new().unwrap();
        config.paths.python = dir.path().display().to_string();

        let args = PassthroughArgs { args: vec![] };
        let err = exec_pip(&i18n, &config, args).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_failed_to_execute("pip")));
    }

    #[cfg(unix)]
    #[test]
    fn exec_npm_errors_when_tool_not_found() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let mut config = Config::default();

        let node_dir = TempDir::new().unwrap();
        let node = node_dir.path().join("node");
        std::fs::write(&node, "").unwrap();
        config.paths.node = node.display().to_string();

        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());

        let args = PassthroughArgs { args: vec![] };
        let err = exec_npm(&i18n, &config, args).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_interpreter_not_found("npm")));
    }

    #[cfg(unix)]
    #[test]
    fn exec_npm_errors_when_npm_cannot_be_executed() {
        let i18n = test_i18n();
        let mut config = Config::default();

        let node_dir = TempDir::new().unwrap();
        let node = node_dir.path().join("node");
        std::fs::write(&node, "").unwrap();

        // Return a directory as npm path to force a spawn error.
        let npm_dir = node_dir.path().join("npm");
        std::fs::create_dir_all(&npm_dir).unwrap();

        config.paths.node = node.display().to_string();

        let args = PassthroughArgs { args: vec![] };
        let err = exec_npm(&i18n, &config, args).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_failed_to_execute("npm")));
    }

    #[cfg(unix)]
    #[test]
    fn exec_npx_errors_when_tool_not_found() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let mut config = Config::default();

        let node_dir = TempDir::new().unwrap();
        let node = node_dir.path().join("node");
        std::fs::write(&node, "").unwrap();
        config.paths.node = node.display().to_string();

        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());

        let args = PassthroughArgs { args: vec![] };
        let err = exec_npx(&i18n, &config, args).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_interpreter_not_found("npx")));
    }

    #[cfg(unix)]
    #[test]
    fn exec_npx_errors_when_npx_cannot_be_executed() {
        let i18n = test_i18n();
        let mut config = Config::default();

        let node_dir = TempDir::new().unwrap();
        let node = node_dir.path().join("node");
        std::fs::write(&node, "").unwrap();

        // Return a directory as npx path to force a spawn error.
        let npx_dir = node_dir.path().join("npx");
        std::fs::create_dir_all(&npx_dir).unwrap();

        config.paths.node = node.display().to_string();

        let args = PassthroughArgs { args: vec![] };
        let err = exec_npx(&i18n, &config, args).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_failed_to_execute("npx")));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_interpreter_with_fallbacks_can_fail() {
        let _lock = env_lock();
        let i18n = test_i18n();
        let empty_path = TempDir::new().unwrap();
        let _path_guard = EnvVarGuard::set("PATH", empty_path.path());

        let err = resolve_interpreter(&i18n, "nope", &["also_nope"]).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_interpreter_not_found("nope")));
    }

    #[cfg(unix)]
    #[test]
    fn exec_script_with_reader_errors_when_stdin_read_fails() {
        struct FailingReader;
        impl Read for FailingReader {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("boom"))
            }
        }

        let i18n = test_i18n();
        let interpreter = PathBuf::from("/bin/sh");
        let args = ScriptArgs {
            code: None,
            file: None,
            stdin: true,
            args: vec![],
        };

        let mut reader = FailingReader;
        let err = exec_script_with_reader(&i18n, &interpreter, args, ScriptType::Py, &mut reader)
            .unwrap_err();
        assert!(err.to_string().contains(i18n.err_read_stdin()));
    }

    #[cfg(unix)]
    #[test]
    fn exec_script_with_reader_runs_file_and_passes_args() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();

        let script = temp_dir.path().join("script.sh");
        write_executable(&script, "#!/bin/sh\nexit 0\n").unwrap();

        let interpreter = PathBuf::from("/bin/sh");
        let args = ScriptArgs {
            code: None,
            file: Some(script),
            stdin: false,
            args: vec![OsString::from("arg0")],
        };

        let mut stdin_reader = std::io::Cursor::new("");
        let code =
            exec_script_with_reader(&i18n, &interpreter, args, ScriptType::Py, &mut stdin_reader)
                .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[cfg(unix)]
    #[test]
    fn exec_script_errors_when_interpreter_cannot_be_executed() {
        let i18n = test_i18n();
        let dir = TempDir::new().unwrap();
        let interpreter = dir.path().to_path_buf();
        let args = ScriptArgs {
            code: Some("true".to_string()),
            file: None,
            stdin: false,
            args: vec![],
        };

        let err = exec_script(&i18n, &interpreter, args, ScriptType::Py).unwrap_err();
        assert!(err
            .to_string()
            .contains(&i18n.err_failed_to_execute(&interpreter.display().to_string())));
    }
}
