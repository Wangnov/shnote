use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use which::which;

use crate::cli::{InitTarget, Scope};
use crate::config::home_dir;
use crate::i18n::I18n;

/// Embedded shnote rules content
const SHNOTE_RULES: &str = r#"# shnote - Shell Command Wrapper

## 概述

shnote 是一个命令包装器，强制在执行命令前填写 WHAT（做什么）和 WHY（为什么）。这有助于让用户能够快速直观地理解 AI Agent 临时编排的复杂命令。

## 重要规则

**当你需要执行任何 shell 命令时，必须使用 shnote 而不是直接使用 Bash/shell_command 工具。**

> 备注：`--what/--why` 只允许用于执行类子命令：`run`、`py`、`node`、`pip`、`npm`、`npx`。  
> 对于 `config/init/setup/doctor/completions` 等管理命令，请直接运行 `shnote <subcommand>`（不要带 `--what/--why`），否则会报错。

### 正确做法

```bash
# 使用 shnote 执行命令
shnote --what "列出目录文件" --why "查看项目结构" run ls -la

# 执行 Python 脚本
shnote --what "运行测试脚本" --why "验证功能正确性" py -c 'print("test")'

# 执行 Node.js 脚本
shnote --what "处理 JSON 数据" --why "转换配置格式" node -c 'console.log(JSON.stringify({a:1}))'

# 安装 Python 包
shnote --what "安装请求库" --why "HTTP 请求需要" pip install requests

# 安装 Node 包
shnote --what "安装 axios" --why "HTTP 客户端需要" npm install axios
```

### 错误做法

```bash
# 不要直接执行命令
ls -la  # 错误！

# 不要省略 --what 和 --why
shnote run ls -la  # 错误！缺少 --what 和 --why
```

## 命令格式

### run - 执行任意 shell 命令

```bash
shnote --what "<做什么>" --why "<为什么>" run <command> [args...]
```

### py - 执行 Python 脚本

```bash
# 内联代码
shnote --what "<做什么>" --why "<为什么>" py -c '<code>'

# 文件
shnote --what "<做什么>" --why "<为什么>" py -f <script.py>

# 从 stdin 读取
shnote --what "<做什么>" --why "<为什么>" py --stdin <<'EOF'
<多行代码>
EOF
```

### node - 执行 Node.js 脚本

```bash
# 内联代码
shnote --what "<做什么>" --why "<为什么>" node -c '<code>'

# 文件
shnote --what "<做什么>" --why "<为什么>" node -f <script.js>
```

### 内联代码注意事项

在使用 `py -c` 或 `node -c` 执行内联代码时，需要注意引号和转义问题：

**Python f-string 限制**：f-string 表达式内不能包含反斜杠

```bash
# 错误：f-string 内有反斜杠会报语法错误
shnote --what "<做什么>" --why "<为什么>" py -c 'print(f"时间: {datetime.now().strftime(\"%Y-%m-%d\")}")'

# 正确：先将格式字符串赋值给变量
shnote --what "<做什么>" --why "<为什么>" py -c 'from datetime import datetime; fmt="%Y-%m-%d"; print(f"时间: {datetime.now().strftime(fmt)}")'
```

**引号嵌套**：外层用单引号时，内层用双引号（或反之）

```bash
# 正确：外单内双
shnote --what "<做什么>" --why "<为什么>" py -c 'print("Hello World")'
shnote --what "<做什么>" --why "<为什么>" node -c 'console.log("Hello World")'

# 正确：外双内单（需要转义外层引号）
shnote --what "<做什么>" --why "<为什么>" py -c "print('Hello World')"
```

### pip - Python 包管理

使用配置的 Python 环境对应的 pip（内部通过 `python -m pip` 实现）。

```bash
# 安装包
shnote --what "安装 requests" --why "HTTP 请求需要" pip install requests

# 查看已安装的包
shnote --what "查看包列表" --why "检查依赖" pip list

# 卸载包
shnote --what "卸载旧版本" --why "版本冲突" pip uninstall package-name
```

### npm - Node.js 包管理

使用与配置的 node 同目录的 npm。

```bash
# 安装包
shnote --what "安装依赖" --why "项目初始化" npm install

# 安装特定包
shnote --what "安装 axios" --why "HTTP 客户端" npm install axios

# 运行脚本
shnote --what "运行构建" --why "打包发布" npm run build
```

### npx - Node.js 包执行器

使用与配置的 node 同目录的 npx。

```bash
# 执行一次性命令
shnote --what "创建 React 应用" --why "初始化新项目" npx create-react-app my-app

# 运行本地包
shnote --what "运行 eslint" --why "代码检查" npx eslint src/
```

## 推荐：使用 uv 避免污染系统环境

如果用户安装了 [uv](https://github.com/astral-sh/uv)，**强烈建议**使用 `uv run` 或 `uvx` 来执行 Python 脚本和工具，而不是用 `pip` 安装到系统环境。

### uv 的优势

- 自动管理临时虚拟环境，不污染系统 pip
- 包只下载一次，通过符号链接复用
- 支持 PEP 723 内联依赖声明
- 安装速度极快

### 使用 uv run 执行带依赖的脚本

```bash
# 使用 --with 指定依赖（推荐）
# 注意：某些包可能有隐式依赖，如 qrcode 保存图片需要 pillow
shnote --what "生成二维码" --why "创建分享链接" run uv run --with qrcode --with pillow python -c "import qrcode; qrcode.make('hello').save('qr.png')"

# 多个依赖用多个 --with
shnote --what "数据处理" --why "分析CSV文件" run uv run --with pandas --with numpy python script.py

# 执行带 PEP 723 内联依赖的脚本文件
shnote --what "运行数据分析" --why "生成报告" run uv run analysis.py
```

PEP 723 内联依赖示例（script.py）:
```python
# /// script
# dependencies = ["requests", "pandas"]
# ///
import requests
import pandas as pd
# ...
```

### 使用 uvx 执行一次性工具

`uvx` 相当于 `uv tool run`，用于执行一次性 Python CLI 工具：

```bash
# 运行 black 格式化代码
shnote --what "格式化代码" --why "统一代码风格" run uvx black src/

# 运行 ruff 检查代码
shnote --what "检查代码" --why "发现潜在问题" run uvx ruff check .

# 运行 httpie 发送请求
shnote --what "测试 API" --why "验证接口" run uvx httpie GET https://api.example.com/users
```

### 何时使用 pip vs uv

| 场景 | 推荐方式 |
|------|----------|
| 一次性脚本需要依赖 | `uv run --with pkg` |
| 一次性 CLI 工具 | `uvx tool-name` |
| 项目开发，需要持久安装 | `pip install` |
| 没有安装 uv | `pip install` |

## --what 和 --why 的编写规范

### --what（做什么）

- 简洁描述这个命令的目的
- 使用动词开头
- 例如："列出目录文件"、"编译项目"、"运行测试"

### --why（为什么）

- 解释执行这个命令的原因
- 提供上下文信息
- 例如："查看项目结构"、"准备发布版本"、"验证修复是否生效"

## 不需要使用 shnote 的情况

以下命令可以直接使用 Bash 工具执行，不需要通过 shnote：

1. **Agent 自身的操作**：如读取文件、编辑文件等（使用专用工具）

## 长时间运行的命令（使用 pueue）

对于**长时间运行**或**持续运行**的命令，必须通过 pueue 放到后台执行，避免阻塞 Agent。

> 如果环境里没有 `pueue/pueued`，可以先运行 `shnote setup`（会安装到 shnote 的 bin 目录，通常为 `~/.shnote/bin`，并提示如何加入 PATH），或自行安装 pueue。

### 需要使用 pueue 的场景

- 启动开发服务器（`npm run dev`、`python -m http.server`、`cargo run` 等）
- 文件监听/热重载（`npm run watch`、`tsc --watch` 等）
- 长时间编译任务
- 任何预期运行时间超过几秒或持续运行的命令

### pueue 使用格式

```bash
# 添加后台任务
shnote --what "<做什么>" --why "<为什么>" run pueue add -- <command> [args...]

# 查看所有任务状态
shnote --what "查看后台任务" --why "检查服务运行状态" run pueue status

# 查看特定任务日志（注意：pueue status 不接受任务 ID）
shnote --what "查看任务日志" --why "调试服务问题" run pueue log <task_id>

# 停止任务
shnote --what "停止后台任务" --why "关闭服务" run pueue kill <task_id>
```

### pueue 注意事项

**复杂命令的限制**：pueue 对命令的引号处理比较敏感，以下情况建议写成脚本文件：

| 问题场景 | 解决方案 |
|----------|----------|
| 多行命令 | 写成脚本文件再运行 |
| 引号嵌套（如 f-string） | 写成脚本文件再运行 |
| `python` 命令找不到 | 使用完整路径 `/usr/bin/python3` |

```bash
# 错误示例：复杂引号嵌套可能失败
pueue add -- python -c 'print(f"value: {x}")'

# 正确做法：先写脚本文件
echo 'print(f"value: {x}")' > /tmp/script.py
shnote --what "运行后台脚本" --why "避免引号问题" run pueue add -- /usr/bin/python3 /tmp/script.py
```

## 示例场景

### 场景 1：查看系统信息

```bash
shnote --what "查看系统信息" --why "诊断环境问题" run uname -a
```

### 场景 2：启动服务（后台运行）

```bash
shnote --what "启动开发服务器" --why "本地测试新功能" run pueue add -- npm run dev
```

### 场景 3：数据处理（使用 uv）

```bash
# 推荐：使用 uv run，不污染系统环境
shnote --what "分析日志" --why "统计错误" run uv run --with pandas python -c 'import pandas as pd; print(pd.read_csv("log.csv")["error"].sum())'
```

### 场景 4：一次性工具（使用 uvx）

```bash
# 推荐：使用 uvx 运行一次性工具
shnote --what "格式化 JSON" --why "美化配置文件" run uvx python-json-tool < config.json
```

### 场景 5：项目依赖安装

```bash
# 项目开发场景，需要持久安装
shnote --what "安装项目依赖" --why "开发环境初始化" pip install -r requirements.txt
```

### 场景 6：批量操作

```bash
shnote --what "批量重命名文件" --why "统一文件命名规范" run find . -name "*.txt" -exec mv {} {}.bak \;
```

## 输出格式

shnote 会在命令输出前显示 WHAT 和 WHY：

```
WHAT: 列出目录文件
WHY:  查看项目结构
file1.txt
file2.txt
...
```

> 注意：如果你在 `shnote ...` 外层再接管道/过滤（例如 `| tail -5`、`| head -20`、`| grep ...`），这些工具可能会截断/过滤掉前两行，从而导致输出里看不到 `WHAT/WHY`。
> 这不影响 `shnote` 的强制记录：请以实际执行命令里的 `--what` / `--why` 参数为准（它们必须写在子命令前，通常在终端/日志里总能看到）。

这使得 AI Agent 可以轻松追踪每个命令的意图和执行结果。
"#;

/// Marker to identify shnote rules section in append mode
const SHNOTE_MARKER_START: &str = "\n<!-- shnote rules start -->\n";
const SHNOTE_MARKER_END: &str = "\n<!-- shnote rules end -->\n";

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
            migrate_shnote_rules(i18n, &old_claude_md, &target_file)?
        } else {
            false
        };

        if !migrated {
            // No migration needed, just write the rules file
            fs::write(&target_file, SHNOTE_RULES)
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
        append_rules(i18n, &target_file)?;
        println!(
            "{}",
            i18n.init_claude_success(&target_file.display().to_string())
        );
    }

    Ok(())
}

/// Migrate shnote rules from old CLAUDE.md to new rules file.
/// Returns true if migration was performed, false if no old rules found.
fn migrate_shnote_rules(i18n: &I18n, old_file: &Path, new_file: &Path) -> Result<bool> {
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
    fs::write(new_file, SHNOTE_RULES)
        .context(i18n.err_write_file(&new_file.display().to_string()))?;

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
    let codex_dir = base.join(".codex");
    let target_file = codex_dir.join("AGENTS.md");

    // Create directory if needed
    fs::create_dir_all(&codex_dir)
        .context(i18n.err_create_dir(&codex_dir.display().to_string()))?;

    append_rules(i18n, &target_file)?;

    println!(
        "{}",
        i18n.init_codex_success(&target_file.display().to_string())
    );
    Ok(())
}

fn init_gemini(i18n: &I18n, scope: Scope) -> Result<()> {
    let _ = probe_cli_tool(i18n, "gemini");
    let base = get_base_dir(i18n, scope)?;
    let gemini_dir = base.join(".gemini");
    let target_file = gemini_dir.join("GEMINI.md");

    // Create directory if needed
    fs::create_dir_all(&gemini_dir)
        .context(i18n.err_create_dir(&gemini_dir.display().to_string()))?;

    append_rules(i18n, &target_file)?;

    println!(
        "{}",
        i18n.init_gemini_success(&target_file.display().to_string())
    );
    Ok(())
}

fn append_rules(i18n: &I18n, target_file: &PathBuf) -> Result<()> {
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
        new_content.push_str(SHNOTE_RULES);
        new_content.push_str(SHNOTE_MARKER_END);
        new_content.push_str(&content[end_idx..]);

        fs::write(target_file, new_content)
            .context(i18n.err_write_file(&target_file.display().to_string()))?;

        println!("{}", i18n.init_rules_updated());
    } else {
        // Append new rules (rewrite the file to keep behavior deterministic and testable)
        content.push_str(SHNOTE_MARKER_START);
        content.push_str(SHNOTE_RULES);
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
        assert!(SHNOTE_RULES.contains("shnote"));
        assert!(SHNOTE_RULES.contains("--what"));
        assert!(SHNOTE_RULES.contains("--why"));
        assert!(SHNOTE_RULES.len() > 1000); // Rules should be substantial
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

        append_rules(&i18n, &target_file).unwrap();

        assert!(target_file.exists());
        let content = fs::read_to_string(&target_file).unwrap();
        assert!(content.contains(SHNOTE_MARKER_START));
        assert!(content.contains(SHNOTE_MARKER_END));
        assert!(content.contains("shnote"));
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

        append_rules(&i18n, &target_file).unwrap();

        let content = fs::read_to_string(&target_file).unwrap();
        assert!(content.contains("Some content"));
        assert!(content.contains("More content"));
        assert!(!content.contains("OLD RULES"));
        assert!(content.contains(SHNOTE_RULES));
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
        assert_eq!(content, SHNOTE_RULES);
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
        assert_eq!(content, SHNOTE_RULES);

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
        assert_eq!(content, SHNOTE_RULES);

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

        let migrated = migrate_shnote_rules(&i18n, &old_file, &new_file).unwrap();
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

        let migrated = migrate_shnote_rules(&i18n, &old_file, &new_file).unwrap();
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

        let err = migrate_shnote_rules(&i18n, &old_file, &new_file).unwrap_err();
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

        let err = migrate_shnote_rules(&i18n, &old_file, &new_file).unwrap_err();
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

        append_rules(&i18n, &target_file).unwrap();

        let content = fs::read_to_string(&target_file).unwrap();
        assert!(content.contains("before"));
        assert!(content.contains(SHNOTE_RULES));
        assert!(!content.contains("OLD RULES WITHOUT END"));
        assert!(!content.contains("after"));
    }

    #[test]
    fn append_rules_errors_when_read_fails() {
        let i18n = test_i18n();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("dir-as-file");
        fs::create_dir_all(&target_file).unwrap();

        let err = append_rules(&i18n, &target_file).unwrap_err();
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

        let err = append_rules(&i18n, &target_file).unwrap_err();
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

        let err = append_rules(&i18n, &target_file).unwrap_err();
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
        assert_eq!(content, SHNOTE_RULES);

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
