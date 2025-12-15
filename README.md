# shnote

[中文](#中文) | [English](#english)

---

## 中文

`shnote` 是一个轻量级的命令包装器，强制在执行命令前记录 WHAT（做什么）和 WHY（为什么），输出协议化信息，方便用户直观快速地理解 AI Agent 执行的复杂命令（如临时的多行python脚本）。

### 特性

- **强制 WHAT/WHY**：对执行类命令（`run/py/node/pip/npm/npx`）要求在子命令前填写 `--what/--why`
- **协议化输出**：`WHAT:` 和 `WHY:` 输出在最前面，便于解析
- **完全透传**：命令输出不做拦截/改写（stdout/stderr 继承），用户自己决定如何使用 pueue
- **多命令支持**：shell、Python、Node.js，以及 `pip/npm/npx` 透传封装
- **跨平台**：支持 macOS、Linux、Windows
- **国际化**：支持中英双语帮助和消息

### 安装

#### 一键安装

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/wangnov/shnote/main/scripts/install.sh | sh

# Windows (PowerShell)
irm https://raw.githubusercontent.com/wangnov/shnote/main/scripts/install.ps1 | iex
```

#### 从源码安装

```bash
cargo install --path .
```

#### 安装 pueue（可选）

shnote 支持通过 pueue 在后台运行长时间任务。安装 shnote 后运行：

```bash
shnote setup
```

这会将 pueue 和 pueued 安装到 `~/.shnote/bin/`。按提示将此目录添加到 PATH：

```bash
# 添加到 ~/.bashrc 或 ~/.zshrc
export PATH="$HOME/.shnote/bin:$PATH"
```

### 用法

#### Shell 命令

```bash
shnote --what "列出文件" --why "查看项目结构" run ls -la
```

#### Python 脚本

```bash
# 内联代码
shnote --what "打印消息" --why "测试Python" py -c 'print("Hello")'

# 文件
shnote --what "运行脚本" --why "处理数据" py -f script.py

# Heredoc
shnote --what "多行脚本" --why "复杂逻辑" py --stdin <<'EOF'
import sys
print("Python version:", sys.version)
EOF
```

#### Node.js 脚本

```bash
shnote --what "运行Node" --why "处理JSON" node -c 'console.log("Hello")'
```

#### pip / npm / npx（透传）

```bash
shnote --what "查看 pip 版本" --why "确认环境" pip --version
shnote --what "查看 npm 版本" --why "确认环境" npm --version
shnote --what "查看 npx 版本" --why "确认环境" npx --version
```

#### pueue 后台任务（透传）

```bash
shnote --what "后台编译" --why "编译大项目" run pueue add -- cargo build --release
```

### 输出格式

```
WHAT: 列出文件
WHY:  查看项目结构
<命令实际输出...>
```

> 注意：如果你在 `shnote ...` 外层再接管道/过滤（例如 `| tail -5`、`| head -20`、`| grep ...`），这些工具可能会截断/过滤掉前两行，从而导致输出里看不到 `WHAT/WHY`。
> 这不影响 `shnote` 的强制记录：请以实际执行命令里的 `--what` / `--why` 参数为准（它们必须写在子命令前，通常在终端/日志里总能看到）。
>
> 另外：`--what/--why` 只允许用于 `run/py/node/pip/npm/npx`，其他命令（如 `config/init/setup/doctor/completions`）不接受这两个参数。

### 配置

配置文件默认位置：

- macOS/Linux：`~/.shnote/config.toml`
- Windows：`%USERPROFILE%\.shnote\config.toml`

也可以通过 `shnote config path` 查看实际路径。

```bash
# 查看配置
shnote config list

# 获取某个配置
shnote config get python

# 设置配置
shnote config set python /usr/bin/python3
shnote config set shell bash
shnote config set language zh

# 重置配置
shnote config reset

# 查看配置文件路径
shnote config path
```

#### 可配置项

| 键 | 说明 | 默认值 |
|----|------|--------|
| python | Python 解释器路径 | python3 |
| node | Node.js 解释器路径 | node |
| shell | Shell 类型 (auto/sh/bash/zsh/pwsh/cmd) | auto |
| language | 语言 (auto/zh/en) | auto |

### 其他命令

```bash
# 检查环境依赖
shnote doctor

# 安装/更新 pueue 与 pueued 到 shnote 的 bin 目录（macOS/Linux 通常为 ~/.shnote/bin；Windows 为 %USERPROFILE%\.shnote\bin）
# 优先使用内嵌二进制；未内嵌时会联网下载并校验 SHA256
# macOS/Linux 依赖 curl（或 wget）与 shasum；Windows 使用 PowerShell 与 certutil
shnote setup

# 初始化 AI 工具规则
shnote init claude   # 会先检测 claude 版本：>= 2.0.64 写入 ~/.claude/rules/shnote.md（覆盖）；否则写入/更新 ~/.claude/CLAUDE.md（追加/替换标记区块）
shnote init codex    # 写入/更新 ~/.codex/AGENTS.md（追加/替换标记区块）
shnote init gemini   # 写入/更新 ~/.gemini/GEMINI.md（追加/替换标记区块）
```

### Shell 补全

shnote 支持为多种 shell 生成补全脚本。

#### Bash

```bash
# 添加到 ~/.bashrc
eval "$(shnote completions bash)"

# 或者保存到文件
shnote completions bash > ~/.local/share/bash-completion/completions/shnote
```

#### Zsh

```bash
# 添加到 ~/.zshrc
eval "$(shnote completions zsh)"

# 或者保存到文件（确保目录在 $fpath 中）
shnote completions zsh > ~/.zsh/completions/_shnote
```

#### Fish

```bash
shnote completions fish > ~/.config/fish/completions/shnote.fish
```

#### PowerShell

```powershell
# 添加到 PowerShell 配置文件
shnote completions powershell | Out-String | Invoke-Expression
```

#### 支持的 Shell

- `bash` - Bash
- `zsh` - Zsh
- `fish` - Fish
- `powershell` - PowerShell
- `elvish` - Elvish

### 语言支持

支持中英双语。语言检测优先级：

1. `--lang` 命令行参数
2. 配置文件中的 `language`
3. 环境变量 `SHNOTE_LANG`、`LC_ALL`、`LC_MESSAGES`、`LANGUAGE`、`LANG`
4. 默认：English

---

## English

`shnote` is a lightweight command wrapper that enforces WHAT/WHY documentation before executing commands, producing structured output that makes it easy to understand complex commands (like temporary multi-line Python scripts) executed by AI Agents.

### Features

- **Mandatory WHAT/WHY**: Execution commands (`run/py/node/pip/npm/npx`) require `--what/--why` flags before the subcommand
- **Structured Output**: `WHAT:` and `WHY:` are output first for easy parsing
- **Full Passthrough**: Command output is not intercepted/modified (stdout/stderr inherited), users decide how to use pueue
- **Multi-command Support**: Shell, Python, Node.js, plus `pip/npm/npx` passthrough wrappers
- **Cross-platform**: Supports macOS, Linux, Windows
- **Internationalization**: Supports English and Chinese help/messages

### Installation

#### One-line Install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/wangnov/shnote/main/scripts/install.sh | sh

# Windows (PowerShell)
irm https://raw.githubusercontent.com/wangnov/shnote/main/scripts/install.ps1 | iex
```

#### From Source

```bash
cargo install --path .
```

#### Install pueue (Optional)

shnote supports running long-running tasks in the background via pueue. After installing shnote, run:

```bash
shnote setup
```

This installs pueue and pueued to `~/.shnote/bin/`. Add this directory to your PATH as prompted:

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.shnote/bin:$PATH"
```

### Usage

#### Shell Commands

```bash
shnote --what "List files" --why "Check project structure" run ls -la
```

#### Python Scripts

```bash
# Inline code
shnote --what "Print message" --why "Test Python" py -c 'print("Hello")'

# File
shnote --what "Run script" --why "Process data" py -f script.py

# Heredoc
shnote --what "Multi-line script" --why "Complex logic" py --stdin <<'EOF'
import sys
print("Python version:", sys.version)
EOF
```

#### Node.js Scripts

```bash
shnote --what "Run Node" --why "Process JSON" node -c 'console.log("Hello")'
```

#### pip / npm / npx (Passthrough)

```bash
shnote --what "Check pip version" --why "Verify environment" pip --version
shnote --what "Check npm version" --why "Verify environment" npm --version
shnote --what "Check npx version" --why "Verify environment" npx --version
```

#### pueue Background Tasks (Passthrough)

```bash
shnote --what "Background build" --why "Compile large project" run pueue add -- cargo build --release
```

### Output Format

```
WHAT: List files
WHY:  Check project structure
<actual command output...>
```

> Note: If you pipe `shnote ...` through filters like `| tail -5`, `| head -20`, or `| grep ...`, these tools may truncate/filter the first two lines, hiding the `WHAT/WHY` output.
> This doesn't affect shnote's mandatory documentation: the `--what` / `--why` parameters in the actual command line (which must appear before the subcommand) are always visible in the terminal/logs.
>
> Also: `--what/--why` are only allowed for `run/py/node/pip/npm/npx`. Other commands (`config/init/setup/doctor/completions`) don't accept these parameters.

### Configuration

Default config file location:

- macOS/Linux: `~/.shnote/config.toml`
- Windows: `%USERPROFILE%\.shnote\config.toml`

Use `shnote config path` to view the actual path.

```bash
# View config
shnote config list

# Get a config value
shnote config get python

# Set config values
shnote config set python /usr/bin/python3
shnote config set shell bash
shnote config set language en

# Reset config
shnote config reset

# View config file path
shnote config path
```

#### Configuration Keys

| Key | Description | Default |
|-----|-------------|---------|
| python | Python interpreter path | python3 |
| node | Node.js interpreter path | node |
| shell | Shell type (auto/sh/bash/zsh/pwsh/cmd) | auto |
| language | Language (auto/zh/en) | auto |

### Other Commands

```bash
# Check environment dependencies
shnote doctor

# Install/update pueue and pueued to shnote's bin directory (usually ~/.shnote/bin on macOS/Linux; %USERPROFILE%\.shnote\bin on Windows)
# Prefers embedded binaries; downloads and verifies SHA256 when not embedded
# macOS/Linux requires curl (or wget) and shasum; Windows uses PowerShell and certutil
shnote setup

# Initialize AI tool rules
shnote init claude   # Detects claude version: >= 2.0.64 writes to ~/.claude/rules/shnote.md (overwrite); otherwise writes/updates ~/.claude/CLAUDE.md (append/replace marked section)
shnote init codex    # Writes/updates ~/.codex/AGENTS.md (append/replace marked section)
shnote init gemini   # Writes/updates ~/.gemini/GEMINI.md (append/replace marked section)
```

### Shell Completion

shnote can generate completion scripts for various shells.

#### Bash

```bash
# Add to ~/.bashrc
eval "$(shnote completions bash)"

# Or save to a file
shnote completions bash > ~/.local/share/bash-completion/completions/shnote
```

#### Zsh

```bash
# Add to ~/.zshrc
eval "$(shnote completions zsh)"

# Or save to a file (ensure directory is in $fpath)
shnote completions zsh > ~/.zsh/completions/_shnote
```

#### Fish

```bash
shnote completions fish > ~/.config/fish/completions/shnote.fish
```

#### PowerShell

```powershell
# Add to PowerShell profile
shnote completions powershell | Out-String | Invoke-Expression
```

#### Supported Shells

- `bash` - Bash
- `zsh` - Zsh
- `fish` - Fish
- `powershell` - PowerShell
- `elvish` - Elvish

### Language Support

Supports English and Chinese. Language detection priority:

1. `--lang` command line argument
2. `language` in config file
3. Environment variables: `SHNOTE_LANG`, `LC_ALL`, `LC_MESSAGES`, `LANGUAGE`, `LANG`
4. Default: English

---

## License

MIT
