#!/usr/bin/env bash
set -euo pipefail

installer_sh="${1:-target/distrib/shnote-installer.sh}"

if [[ ! -f "$installer_sh" ]]; then
  echo "installer not found: $installer_sh" >&2
  exit 1
fi

tmp_file="$(mktemp)"

awk '
BEGIN {
  in_say_verbose = 0
  inserted = 0
}
{
  print $0

  if ($0 ~ /^say_verbose\(\) \{$/) {
    in_say_verbose = 1
    next
  }

  if (in_say_verbose == 1 && $0 ~ /^\}$/ && inserted == 0) {
    print ""
    print "resolve_color_name_from_config() {"
    print "    local _key=\"$1\""
    print "    local _fallback=\"$2\""
    print "    local _cfg_home=\"${INFERRED_HOME:-$HOME}\""
    print "    local _cfg=\"$_cfg_home/.shnote/config.toml\""
    print "    local _value=\"\""
    print ""
    print "    if [ -f \"$_cfg\" ]; then"
    print "        _value=$(sed -n \"s/^[[:space:]]*${_key}[[:space:]]*=[[:space:]]*\\\"\\([^\\\"]*\\)\\\".*/\\1/p\" \"$_cfg\" | tail -n 1 | tr \"[:upper:]\" \"[:lower:]\")"
    print "    fi"
    print ""
    print "    if [ -n \"$_value\" ]; then"
    print "        echo \"$_value\""
    print "    else"
    print "        echo \"$_fallback\""
    print "    fi"
    print "}"
    print ""
    print "is_color_enabled_from_config() {"
    print "    local _cfg_home=\"${INFERRED_HOME:-$HOME}\""
    print "    local _cfg=\"$_cfg_home/.shnote/config.toml\""
    print "    local _value=\"\""
    print ""
    print "    if [ -f \"$_cfg\" ]; then"
    print "        _value=$(sed -n \"s/^[[:space:]]*color[[:space:]]*=[[:space:]]*//p\" \"$_cfg\" | tail -n 1 | sed \"s/[[:space:]]*#.*$//\" | tr -d \"[:space:]\" | tr \"[:upper:]\" \"[:lower:]\")"
    print "    fi"
    print ""
    print "    case \"$_value\" in"
    print "        false) echo \"0\" ;;"
    print "        *) echo \"1\" ;;"
    print "    esac"
    print "}"
    print ""
    print "color_name_to_escape() {"
    print "    case \"$1\" in"
    print "        black) echo \"30\" ;;"
    print "        red) echo \"31\" ;;"
    print "        green) echo \"32\" ;;"
    print "        yellow) echo \"33\" ;;"
    print "        blue) echo \"34\" ;;"
    print "        magenta) echo \"38;5;5\" ;;"
    print "        cyan) echo \"38;5;6\" ;;"
    print "        white) echo \"37\" ;;"
    print "        bright_black) echo \"90\" ;;"
    print "        bright_red) echo \"91\" ;;"
    print "        bright_green) echo \"92\" ;;"
    print "        bright_yellow) echo \"93\" ;;"
    print "        bright_blue) echo \"94\" ;;"
    print "        bright_magenta) echo \"38;5;13\" ;;"
    print "        bright_cyan) echo \"38;5;14\" ;;"
    print "        bright_white) echo \"97\" ;;"
    print "        default) echo \"\" ;;"
    print "        *) echo \"\" ;;"
    print "    esac"
    print "}"
    print ""
    print "render_install_success_banner() {"
    print "    local _install_dir=\"$1\""
    print "    local _gray=\"$(printf \"\\033[90m\")\""
    print "    local _white=\"$(printf \"\\033[97m\")\""
    print "    local _green=\"$(printf \"\\033[32m\")\""
    print "    local _bold=\"$(printf \"\\033[1m\")\""
    print "    local _dim=\"$(printf \"\\033[2m\")\""
    print "    local _reset=\"$(printf \"\\033[0m\")\""
    print ""
    print "    local _what_color_name"
    print "    local _why_color_name"
    print "    _what_color_name=\"$(resolve_color_name_from_config \"what_color\" \"cyan\")\""
    print "    _why_color_name=\"$(resolve_color_name_from_config \"why_color\" \"magenta\")\""
    print ""
    print "    local _what_color_code"
    print "    local _why_color_code"
    print "    _what_color_code=\"$(color_name_to_escape \"$_what_color_name\")\""
    print "    _why_color_code=\"$(color_name_to_escape \"$_why_color_name\")\""
    print ""
    print "    local _display_install_dir=\"$_install_dir\""
    print "    if [ -n \"${INFERRED_HOME:-}\" ]; then"
    print "        case \"$_display_install_dir\" in"
    print "            \"${INFERRED_HOME}\"/*)"
    print "                _display_install_dir=\"${INFERRED_HOME_EXPRESSION}${_display_install_dir#${INFERRED_HOME}}\""
    print "                ;;"
    print "        esac"
    print "    fi"
    print ""
    print "    local _what_label=\"WHAT\""
    print "    local _why_label=\"WHY\""
    print "    local _color_enabled"
    print "    _color_enabled=\"$(is_color_enabled_from_config)\""
    print "    if [ \"$_color_enabled\" != \"1\" ]; then"
    print "        _gray=\"\""
    print "        _white=\"\""
    print "        _green=\"\""
    print "        _bold=\"\""
    print "        _dim=\"\""
    print "        _reset=\"\""
    print "    fi"
    print "    if [ \"$_color_enabled\" = \"1\" ] && [ -n \"$_what_color_code\" ]; then"
    print "        _what_label=\"$(printf \"\\033[2;%smWHAT\\033[0m\" \"$_what_color_code\")\""
    print "    fi"
    print "    if [ \"$_color_enabled\" = \"1\" ] && [ -n \"$_why_color_code\" ]; then"
    print "        _why_label=\"$(printf \"\\033[2;%smWHY\\033[0m\" \"$_why_color_code\")\""
    print "    fi"
    print ""
    print "    say \"${_green}${_bold}•${_reset} ${_bold}Ran${_reset} ${_white}shnote${_reset} ${_white}--what${_reset} ${_gray}\\\"Confirm shnote is ready\\\"${_reset} ${_white}--why${_reset} ${_gray}\\\"Show the installed binary path\\\"${_reset}\""
    print "    say \"  │ ${_white}run${_reset} ${_gray}\\\"command -v shnote\\\"${_reset}\""
    print "    say \"  │ ${_dim}Installed into ${_display_install_dir} success!${_reset}\""
    print "    say \"    ${_what_label}${_gray}: Confirm shnote is ready${_reset}\""
    print "    say \"    ${_why_label}${_gray}: Show the installed binary path${_reset}\""
    print "}"

    inserted = 1
    in_say_verbose = 0
  }
}
END {
  if (inserted == 0) {
    exit 2
  }
}
' "$installer_sh" > "$tmp_file"

mv "$tmp_file" "$installer_sh"

tmp_file="$(mktemp)"

awk '
{
  lines[++n] = $0
}
END {
  anchor = 0
  for (i = 1; i <= n; i++) {
    if (lines[i] ~ /^[[:space:]]*# Avoid modifying.*PATH/) {
      anchor = i
      break
    }
  }

  if (anchor == 0) {
    exit 3
  }

  success_line = 0
  for (i = anchor - 1; i >= 1; i--) {
    if (lines[i] ~ /^[[:space:]]*$/) {
      continue
    }
    if (lines[i] ~ /^[[:space:]]*say ".*"$/) {
      success_line = i
    }
    break
  }

  for (i = 1; i < anchor; i++) {
    if (i == success_line) {
      continue
    }
    print lines[i]
  }

  print "    render_install_success_banner \"$_install_dir\""

  for (i = anchor; i <= n; i++) {
    print lines[i]
  }
}
' "$installer_sh" > "$tmp_file"

mv "$tmp_file" "$installer_sh"

chmod +x "$installer_sh"
