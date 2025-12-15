# shnote

`shnote` 是一个轻量级的命令包装器，强制在执行命令前记录 WHAT（做什么）和 WHY（为什么），输出协议化信息，方便用户直观快速地理解 AI Agent 执行的复杂命令（如临时的多行python脚本）。

## 特性

- **强制 WHAT/WHY**：对执行类命令（`run/py/node/pip/npm/npx`）要求在子命令前填写 `--what/--why`
- **协议化输出**：`WHAT:` 和 `WHY:` 输出在最前面，便于解析
- **完全透传**：命令输出不做拦截/改写（stdout/stderr 继承），用户自己决定如何使用 pueue
- **多命令支持**：shell、Python、Node.js，以及 `pip/npm/npx` 透传封装
- **跨平台**：支持 macOS、Linux、Windows

## 安装

```bash
cargo install --path .
```

## 用法

### Shell 命令

```bash
shnote --what "列出文件" --why "查看项目结构" run ls -la
```

### Python 脚本

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

### Node.js 脚本

```bash
shnote --what "运行Node" --why "处理JSON" node -c 'console.log("Hello")'
```

### pip / npm / npx（透传）

```bash
shnote --what "查看 pip 版本" --why "确认环境" pip --version
shnote --what "查看 npm 版本" --why "确认环境" npm --version
shnote --what "查看 npx 版本" --why "确认环境" npx --version
```

### pueue 后台任务（透传）

```bash
shnote --what "后台编译" --why "编译大项目" run pueue add -- cargo build --release
```

## 输出格式

```
WHAT: 列出文件
WHY:  查看项目结构
<命令实际输出...>
```

> 注意：如果你在 `shnote ...` 外层再接管道/过滤（例如 `| tail -5`、`| head -20`、`| grep ...`），这些工具可能会截断/过滤掉前两行，从而导致输出里看不到 `WHAT/WHY`。
> 这不影响 `shnote` 的强制记录：请以实际执行命令里的 `--what` / `--why` 参数为准（它们必须写在子命令前，通常在终端/日志里总能看到）。
>
> 另外：`--what/--why` 只允许用于 `run/py/node/pip/npm/npx`，其他命令（如 `config/init/setup/doctor/completions`）不接受这两个参数。

## 配置

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

### 可配置项

| 键 | 说明 | 默认值 |
|----|------|--------|
| python | Python 解释器路径 | python3 |
| node | Node.js 解释器路径 | node |
| shell | Shell 类型 (auto/sh/bash/zsh/pwsh/cmd) | auto |
| language | 语言 (auto/zh/en) | auto |

## 其他命令

```bash
# 检查环境依赖
shnote doctor

# 安装/更新 pueue 与 pueued 到 shnote 的 bin 目录（macOS/Linux 通常为 ~/.shnote/bin；Windows 为 %USERPROFILE%\.shnote\bin）
# 优先使用内嵌二进制；未内嵌时会联网下载并校验 SHA256
# macOS/Linux 依赖 curl（或 wget）与 shasum；Windows 使用 PowerShell 与 certutil
shnote setup

# 初始化 AI 工具规则
shnote init claude   # 写入 ~/.claude/rules/shnote.md（覆盖）
shnote init codex    # 写入/更新 ~/.codex/AGENTS.md（追加/替换标记区块）
shnote init gemini   # 写入/更新 ~/.gemini/GEMINI.md（追加/替换标记区块）
```

## Shell 补全

shnote 支持为多种 shell 生成补全脚本。

### Bash

```bash
# 添加到 ~/.bashrc
eval "$(shnote completions bash)"

# 或者保存到文件
shnote completions bash > ~/.local/share/bash-completion/completions/shnote
```

### Zsh

```bash
# 添加到 ~/.zshrc
eval "$(shnote completions zsh)"

# 或者保存到文件（确保目录在 $fpath 中）
shnote completions zsh > ~/.zsh/completions/_shnote
```

### Fish

```bash
shnote completions fish > ~/.config/fish/completions/shnote.fish
```

### PowerShell

```powershell
# 添加到 PowerShell 配置文件
shnote completions powershell | Out-String | Invoke-Expression
```

### 支持的 Shell

- `bash` - Bash
- `zsh` - Zsh
- `fish` - Fish
- `powershell` - PowerShell
- `elvish` - Elvish

## 语言支持

支持中英双语。语言检测优先级：

1. `--lang` 命令行参数
2. 配置文件中的 `language`
3. 环境变量 `SHNOTE_LANG`、`LC_ALL`、`LC_MESSAGES`、`LANGUAGE`、`LANG`
4. 默认：English

## License

MIT
