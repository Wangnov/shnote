use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::InitTarget;
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

pub fn run_init(i18n: &I18n, target: InitTarget) -> Result<()> {
    match target {
        InitTarget::Claude => init_claude(i18n),
        InitTarget::Codex => init_codex(i18n),
        InitTarget::Gemini => init_gemini(i18n),
    }
}

fn init_claude(i18n: &I18n) -> Result<()> {
    let home = home_dir().context(i18n.err_home_dir())?;
    let rules_dir = home.join(".claude").join("rules");
    let target_file = rules_dir.join("shnote.md");

    // Create directory if needed
    fs::create_dir_all(&rules_dir).context(i18n.err_create_dir(&rules_dir.display().to_string()))?;

    // Write rules file (overwrite)
    fs::write(&target_file, SHNOTE_RULES)
        .context(i18n.err_write_file(&target_file.display().to_string()))?;

    println!(
        "{}",
        i18n.init_claude_success(&target_file.display().to_string())
    );
    Ok(())
}

fn init_codex(i18n: &I18n) -> Result<()> {
    let home = home_dir().context(i18n.err_home_dir())?;
    let codex_dir = home.join(".codex");
    let target_file = codex_dir.join("AGENTS.md");

    // Create directory if needed
    fs::create_dir_all(&codex_dir).context(i18n.err_create_dir(&codex_dir.display().to_string()))?;

    append_rules(i18n, &target_file)?;

    println!(
        "{}",
        i18n.init_codex_success(&target_file.display().to_string())
    );
    Ok(())
}

fn init_gemini(i18n: &I18n) -> Result<()> {
    let home = home_dir().context(i18n.err_home_dir())?;
    let gemini_dir = home.join(".gemini");
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
        fs::read_to_string(target_file).context(i18n.err_read_file(
            &target_file.display().to_string(),
        ))?
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

        fs::write(target_file, content).context(i18n.err_write_file(&target_file.display().to_string()))?;

        println!("{}", i18n.init_rules_appended());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::test_support::{env_lock, EnvVarGuard};
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

    #[test]
    fn init_claude_creates_file() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        let i18n = test_i18n();
        init_claude(&i18n).unwrap();

        let rules_file = temp_dir.path().join(".claude/rules/shnote.md");
        assert!(rules_file.exists());
        let content = fs::read_to_string(rules_file).unwrap();
        assert_eq!(content, SHNOTE_RULES);
    }

    #[test]
    fn init_claude_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = test_i18n();
        let err = init_claude(&i18n).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_home_dir()));
    }

    #[test]
    fn init_claude_errors_when_create_dir_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Make ~/.claude a file so ~/.claude/rules cannot be created.
        fs::write(temp_dir.path().join(".claude"), "not a dir").unwrap();

        let i18n = test_i18n();
        let err = init_claude(&i18n).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_create_dir(
            &temp_dir.path().join(".claude/rules").display().to_string()
        )));
    }

    #[test]
    fn init_claude_errors_when_write_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        fs::create_dir_all(temp_dir.path().join(".claude/rules/shnote.md")).unwrap();

        let i18n = test_i18n();
        let err = init_claude(&i18n).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_write_file(
            &temp_dir
                .path()
                .join(".claude/rules/shnote.md")
                .display()
                .to_string()
        )));
    }

    #[test]
    fn init_codex_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = test_i18n();
        let err = init_codex(&i18n).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_home_dir()));
    }

    #[test]
    fn init_gemini_errors_when_home_dir_missing() {
        let _lock = env_lock();
        let _home_guard = EnvVarGuard::remove("HOME");
        let _userprofile_guard = EnvVarGuard::remove("USERPROFILE");

        let i18n = test_i18n();
        let err = init_gemini(&i18n).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_home_dir()));
    }

    #[test]
    fn init_codex_errors_when_create_dir_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Make ~/.codex a file so ~/.codex cannot be created.
        fs::write(temp_dir.path().join(".codex"), "not a dir").unwrap();

        let i18n = test_i18n();
        let err = init_codex(&i18n).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_create_dir(
            &temp_dir.path().join(".codex").display().to_string()
        )));
    }

    #[test]
    fn init_gemini_errors_when_create_dir_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        // Make ~/.gemini a file so ~/.gemini cannot be created.
        fs::write(temp_dir.path().join(".gemini"), "not a dir").unwrap();

        let i18n = test_i18n();
        let err = init_gemini(&i18n).unwrap_err();
        assert!(err.to_string().contains(&i18n.err_create_dir(
            &temp_dir.path().join(".gemini").display().to_string()
        )));
    }

    #[test]
    fn init_codex_errors_when_append_rules_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        fs::create_dir_all(temp_dir.path().join(".codex/AGENTS.md")).unwrap();

        let i18n = test_i18n();
        let err = init_codex(&i18n).unwrap_err();
        let target_file = temp_dir.path().join(".codex/AGENTS.md");
        assert!(err
            .to_string()
            .contains(&i18n.err_read_file(&target_file.display().to_string())));
    }

    #[test]
    fn init_gemini_errors_when_append_rules_fails() {
        let _lock = env_lock();
        let temp_dir = TempDir::new().unwrap();
        let _home_guard = EnvVarGuard::set("HOME", temp_dir.path());

        fs::create_dir_all(temp_dir.path().join(".gemini/GEMINI.md")).unwrap();

        let i18n = test_i18n();
        let err = init_gemini(&i18n).unwrap_err();
        let target_file = temp_dir.path().join(".gemini/GEMINI.md");
        assert!(err
            .to_string()
            .contains(&i18n.err_read_file(&target_file.display().to_string())));
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
        assert!(err
            .to_string()
            .contains(&i18n.err_read_file(&target_file.display().to_string())));
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
}
