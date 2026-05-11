# Minimal Nushell bootstrap script for Warp.
# Sourced via RC-file bootstrap after the init shell script has sent InitShell.
# Comments are stripped by the bootstrap loader.

# Handle initial working directory if provided.
if ($env.WARP_INITIAL_WORKING_DIR? | is-not-empty) { cd $env.WARP_INITIAL_WORKING_DIR; hide-env WARP_INITIAL_WORKING_DIR }

# Helper: send a hex-encoded JSON message via DCS to Warp.
def warp_send_json_message [msg: string] { let hex = ($msg | into binary | encode hex | str downcase); print -n ((char -i 0x1b) + (char -i 0x50) + (char -i 0x24) + "d" + $hex + (char -i 0x9c)) }

# Escape a string for safe embedding in JSON values.
def warp_escape_json [s: string] { $s | str replace --all '\\' '\\\\' | str replace --all '"' '\\"' | str replace --all "\n" '\\n' | str replace --all "\t" '\\t' | str replace --all "\r" '\\r' }

# Block counter for Warp block IDs.
$env.WARP_BLOCK_ID = 0

# Precmd hook: tell Warp about cwd, git info, and session state before each prompt.
def warp_precmd [] {
    let exit_code = if ($env.LAST_EXIT_CODE? | is-not-empty) { $env.LAST_EXIT_CODE } else { 0 }
    if ($env.WARP_BOOTSTRAPPED? == "1") { warp_send_json_message $'{"hook": "CommandFinished", "value": {"exit_code": ($exit_code), "next_block_id": "precmd-($env.WARP_SESSION_ID)-($env.WARP_BLOCK_ID)"}}' }
    $env.WARP_BLOCK_ID = ($env.WARP_BLOCK_ID | into int) + 1
    let escaped_pwd = (warp_escape_json $env.PWD)
    let git_branch = (try { ^git symbolic-ref --short HEAD | str trim } catch { "" })
    let git_head = if ($git_branch | is-empty) { try { ^git rev-parse --short HEAD | str trim } catch { "" } } else { $git_branch }
    let escaped_git_head = (warp_escape_json $git_head)
    let escaped_git_branch = (warp_escape_json $git_branch)
    let escaped_virtual_env = if ($env.VIRTUAL_ENV? | is-not-empty) { warp_escape_json $env.VIRTUAL_ENV } else { "" }
    let escaped_conda_env = if ($env.CONDA_DEFAULT_ENV? | is-not-empty) { warp_escape_json $env.CONDA_DEFAULT_ENV } else { "" }
    warp_send_json_message $'{"hook": "Precmd", "value": {"pwd": "($escaped_pwd)", "ps1": "", "rprompt": "", "git_head": "($escaped_git_head)", "git_branch": "($escaped_git_branch)", "virtual_env": "($escaped_virtual_env)", "conda_env": "($escaped_conda_env)", "node_version": "", "session_id": ($env.WARP_SESSION_ID)}}'
}

# Preexec hook: tell Warp a command is about to run.
def warp_preexec [command_text: string] {
    let escaped = (warp_escape_json $command_text)
    warp_send_json_message $'{"hook": "Preexec", "value": {"command": "($escaped)"}}'
}

# Wire hooks into Nu config.
$env.config = ($env.config | upsert hooks { |c|
    let existing = ($c | get --optional hooks | default {})
    let existing_pre_prompt = ($existing | get --optional pre_prompt | default [])
    let existing_pre_execution = ($existing | get --optional pre_execution | default [])
    $existing | upsert pre_prompt ($existing_pre_prompt | append {|| warp_precmd }) | upsert pre_execution ($existing_pre_execution | append {|ctx| warp_preexec ($ctx | get --optional commandline | default "") })
})

# Mark the session as bootstrapped.
$env.WARP_BOOTSTRAPPED = "1"

# Tell Warp we are ready and send initial Precmd.
warp_send_json_message '{"hook": "Bootstrapped"}'
warp_precmd
