use crate::cli::parser::{Cli, Shell};
use crate::utils::{ParaError, Result};
use clap::CommandFactory;
use clap_complete::{generate, shells};

pub struct ShellCompletionGenerator;

impl ShellCompletionGenerator {
    pub fn generate_basic_completion(shell: Shell) -> Result<String> {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();

        match shell {
            Shell::Bash => generate(shells::Bash, &mut cmd, "para", &mut buf),
            Shell::Zsh => generate(shells::Zsh, &mut cmd, "para", &mut buf),
            Shell::Fish => generate(shells::Fish, &mut cmd, "para", &mut buf),
        }

        String::from_utf8(buf).map_err(|e| {
            ParaError::invalid_args(format!("UTF-8 error generating completion: {}", e))
        })
    }

    pub fn generate_enhanced_completion(shell: Shell) -> Result<String> {
        let basic = Self::generate_basic_completion(shell.clone())?;
        let dynamic = Self::generate_dynamic_completion(shell)?;

        Ok(format!("{}\n\n{}", basic, dynamic))
    }

    fn generate_dynamic_completion(shell: Shell) -> Result<String> {
        match shell {
            Shell::Bash => Ok(Self::generate_bash_dynamic()),
            Shell::Zsh => Ok(Self::generate_zsh_dynamic()),
            Shell::Fish => Ok(Self::generate_fish_dynamic()),
        }
    }

    fn generate_bash_dynamic() -> String {
        r#"
# Para completion helper functions for bash

_para_complete_sessions() {
    local sessions
    if command -v para >/dev/null 2>&1; then
        sessions=$(para list --quiet 2>/dev/null | grep -o '^[a-zA-Z0-9_-]*' || true)
        if [[ -n "$sessions" ]]; then
            COMPREPLY=($(compgen -W "$sessions" -- "$1"))
        fi
    fi
}

_para_complete_archived_sessions() {
    local sessions
    if command -v para >/dev/null 2>&1; then
        sessions=$(para list --archived --quiet 2>/dev/null | grep -o '^[a-zA-Z0-9_-]*' || true)
        if [[ -n "$sessions" ]]; then
            COMPREPLY=($(compgen -W "$sessions" -- "$1"))
        fi
    fi
}

_para_complete_branches() {
    local branches
    if git rev-parse --git-dir >/dev/null 2>&1; then
        branches=$(git branch -a 2>/dev/null | sed 's/^[* ]*//' | grep -v '^remotes/origin/HEAD' | sed 's|^remotes/origin/||' | sort -u)
        if [[ -n "$branches" ]]; then
            COMPREPLY=($(compgen -W "$branches" -- "$1"))
        fi
    fi
}


_para_complete_shells() {
    local shells="bash zsh fish"
    COMPREPLY=($(compgen -W "$shells" -- "$1"))
}

_para_complete_config_commands() {
    local config_commands="setup auto show edit reset"
    COMPREPLY=($(compgen -W "$config_commands" -- "$1"))
}

_para_complete_task_files() {
    local task_files
    task_files=$(find . -maxdepth 1 \( -name "TASK_*.md" -o -name "*.md" -o -name "*.txt" \) 2>/dev/null | sed 's|^\./||')
    if [[ -n "$task_files" ]]; then
        COMPREPLY=($(compgen -W "$task_files" -- "$1"))
    fi
    # Also include regular file completion
    COMPREPLY+=($(compgen -f -- "$1"))
}

# Enhanced para completion
_para_completion() {
    local cur prev words cword
    _init_completion || return

    # Handle different completion contexts
    case "${prev}" in
        # Session completions
        resume|cancel)
            _para_complete_sessions "${cur}"
            return 0
            ;;
        recover)
            _para_complete_archived_sessions "${cur}"
            return 0
            ;;
        # Option completions
        --file|-f)
            _para_complete_task_files "${cur}"
            return 0
            ;;
        --branch)
            _para_complete_branches "${cur}"
            return 0
            ;;
        --target)
            _para_complete_branches "${cur}"
            return 0
            ;;
    esac

    # Command-specific completions
    case "${words[1]}" in
        finish)
            # Third argument for finish can be session name
            if [[ $cword -eq 3 ]]; then
                _para_complete_sessions "${cur}"
                return 0
            fi
            ;;
        completion)
            _para_complete_shells "${cur}"
            return 0
            ;;
        config)
            _para_complete_config_commands "${cur}"
            return 0
            ;;
        dispatch)
            # Handle dispatch file completion
            if [[ "${cur}" == --* ]]; then
                COMPREPLY=($(compgen -W "--file --dangerously-skip-permissions --help" -- "${cur}"))
            fi
            ;;
        *)
            # Default to command completion if no command selected
            if [[ $cword -eq 1 ]]; then
                local commands="start dispatch finish cancel clean list resume recover config completion help"
                COMPREPLY=($(compgen -W "$commands" -- "${cur}"))
            fi
            ;;
    esac
}

# Register the enhanced completion
complete -F _para_completion para
"#.to_string()
    }

    fn generate_zsh_dynamic() -> String {
        r#"
# Para completion helper functions for zsh
_para_sessions() {
    local sessions
    sessions=(${(f)"$(para list --quiet 2>/dev/null | grep -o '^[a-zA-Z0-9_-]*' 2>/dev/null || true)"})
    _describe 'active sessions' sessions
}

_para_archived_sessions() {
    local sessions
    sessions=(${(f)"$(para list --archived --quiet 2>/dev/null | grep -o '^[a-zA-Z0-9_-]*' 2>/dev/null || true)"})
    _describe 'archived sessions' sessions
}

_para_branches() {
    local branches
    if git rev-parse --git-dir >/dev/null 2>&1; then
        branches=(${(f)"$(git branch -a 2>/dev/null | sed 's/^[* ]*//' | grep -v '^remotes/origin/HEAD' | sed 's|^remotes/origin/||' | sort -u)"})
        _describe 'git branches' branches
    fi
}


_para_shells() {
    local shells
    shells=(
        'bash:Bash shell completion'
        'zsh:Zsh shell completion'
        'fish:Fish shell completion'
    )
    _describe 'shell types' shells
}

_para_config_commands() {
    local config_commands
    config_commands=(
        'setup:Interactive configuration wizard'
        'auto:Auto-detect and configure IDE'
        'show:Show current configuration'
        'edit:Edit configuration file'
        'reset:Reset configuration to defaults'
    )
    _describe 'config commands' config_commands
}

_para_task_files() {
    local task_files
    task_files=(${(f)"$(find . -maxdepth 1 \( -name "TASK_*.md" -o -name "*.md" -o -name "*.txt" \) 2>/dev/null | sed 's|^\./||')"})
    _describe 'task files' task_files
}

# Enhanced para completion
_para() {
    local context state line
    typeset -A opt_args

    _arguments \
        '1: :_para_commands' \
        '*::arg:->args' \
        && return 0

    case $state in
        args)
            case $words[1] in
                resume|cancel)
                    _para_sessions
                    ;;
                recover)
                    _para_archived_sessions
                    ;;
                finish)
                    case $words[CURRENT-1] in
                        --branch)
                            _para_branches
                            ;;
                        *)
                            if [[ $CURRENT -eq 4 ]]; then
                                _para_sessions
                            fi
                            ;;
                    esac
                    ;;
                dispatch)
                    case $words[CURRENT-1] in
                        --file|-f)
                            _para_task_files
                            ;;
                        *)
                            if [[ $words[CURRENT] == --* ]]; then
                                _arguments '--file[Read prompt from file]:file:_files'
                            fi
                            ;;
                    esac
                    ;;
                completion)
                    _para_shells
                    ;;
                config)
                    _para_config_commands
                    ;;
            esac
            ;;
    esac
}

_para_commands() {
    local commands
    commands=(
        'start:Create session with optional name'
        'dispatch:Start Claude Code session with prompt'
        'finish:Squash all changes into single commit'
        'cancel:Cancel session (moves to archive)'
        'clean:Remove all active sessions'
        'list:List active sessions'
        'resume:Resume session in IDE'
        'recover:Recover cancelled session from archive'
        'config:Setup configuration'
        'completion:Generate shell completion script'
    )
    _describe 'para commands' commands
}

# Register the completion
compdef _para para
"#.to_string()
    }

    fn generate_fish_dynamic() -> String {
        r#"
# Para completion helper functions for fish shell
function __para_sessions
    para list --quiet 2>/dev/null | string match -r '^[a-zA-Z0-9_-]*' 2>/dev/null
end

function __para_archived_sessions
    para list --archived --quiet 2>/dev/null | string match -r '^[a-zA-Z0-9_-]*' 2>/dev/null
end

function __para_branches
    if git rev-parse --git-dir >/dev/null 2>&1
        git branch -a 2>/dev/null | sed 's/^[* ]*//' | grep -v '^remotes/origin/HEAD' | sed 's|^remotes/origin/||' | sort -u
    end
end

# Enhanced Para Dynamic Completions

# 1. SESSION COMPLETIONS
# para resume <session-name>
complete -f -c para -n "__fish_para_using_subcommand resume" -a "(__para_sessions)" -d "Active session"

# para cancel <session-name>  
complete -f -c para -n "__fish_para_using_subcommand cancel" -a "(__para_sessions)" -d "Active session"

# para recover <session-name>
complete -f -c para -n "__fish_para_using_subcommand recover" -a "(__para_archived_sessions)" -d "Archived session"

# para finish [message] [session-name] - session name is second argument after message
function __para_finish_needs_session
    set -l cmd (commandline -opc)
    test (count $cmd) -ge 3
end
complete -f -c para -n "__fish_para_using_subcommand finish; and __para_finish_needs_session" -a "(__para_sessions)" -d "Session to finish"

# 2. BRANCH COMPLETIONS  
# para finish --branch <branch>
complete -f -c para -n "__fish_para_using_subcommand finish" -l branch -a "(__para_branches)" -d "Custom branch name"

# 4. FILE COMPLETIONS
# para dispatch --file <file>
complete -c para -n "__fish_para_using_subcommand dispatch" -s f -l file -F -d "Prompt file"

# 5. SHELL COMPLETIONS  
# para completion <shell>
complete -f -c para -n "__fish_para_using_subcommand completion" -a "bash zsh fish" -d "Shell type"

# 6. CONFIG SUBCOMMAND COMPLETIONS
# para config <subcommand>
complete -f -c para -n "__fish_para_using_subcommand config" -a "setup auto show edit reset" -d "Config operation"

# 7. SPECIAL COMPLETIONS FOR TASK FILES
# Enhanced file completion for dispatch that prioritizes .md files and TASK_* files
function __para_task_files
    # Prioritize TASK_* files and .md files
    find . -maxdepth 1 \( -name "TASK_*.md" -o -name "*.md" -o -name "*.txt" \) 2>/dev/null | sed 's|^\./||'
end
complete -c para -n "__fish_para_using_subcommand dispatch" -s f -l file -a "(__para_task_files)" -d "Task or prompt file"
"#.to_string()
    }

    pub fn get_installation_instructions(shell: Shell) -> String {
        match shell {
            Shell::Bash => r#"# Installation instructions for Bash completion:

# Option 1: Install system-wide (requires root)
sudo mkdir -p /etc/bash_completion.d
para completion bash | sudo tee /etc/bash_completion.d/para

# Option 2: Install for current user
mkdir -p ~/.local/share/bash-completion/completions
para completion bash > ~/.local/share/bash-completion/completions/para

# Option 3: Add to your ~/.bashrc
echo 'eval "$(para completion bash)"' >> ~/.bashrc

# Then reload your shell:
source ~/.bashrc"#
                .to_string(),
            Shell::Zsh => r#"# Installation instructions for Zsh completion:

# Option 1: Install to a directory in your $fpath
mkdir -p ~/.zsh_completions
para completion zsh > ~/.zsh_completions/_para
# Add to ~/.zshrc: fpath=(~/.zsh_completions $fpath)

# Option 2: Install system-wide (requires root)
sudo mkdir -p /usr/local/share/zsh/site-functions
para completion zsh | sudo tee /usr/local/share/zsh/site-functions/_para

# Option 3: Add to your ~/.zshrc
echo 'eval "$(para completion zsh)"' >> ~/.zshrc

# Then reload your shell:
source ~/.zshrc"#
                .to_string(),
            Shell::Fish => r#"# Installation instructions for Fish completion:

# Option 1: Install for current user
mkdir -p ~/.config/fish/completions
para completion fish > ~/.config/fish/completions/para.fish

# Option 2: Install system-wide (requires root)
sudo mkdir -p /usr/share/fish/vendor_completions.d
para completion fish | sudo tee /usr/share/fish/vendor_completions.d/para.fish

# Fish will automatically load the completion on next shell start"#
                .to_string(),
        }
    }
}
