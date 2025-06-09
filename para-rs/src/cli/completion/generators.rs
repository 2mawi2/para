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
            Shell::PowerShell => generate(shells::PowerShell, &mut cmd, "para", &mut buf),
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
            Shell::PowerShell => Ok(Self::generate_powershell_dynamic()),
        }
    }

    fn generate_bash_dynamic() -> String {
        r#"
# Para dynamic completion functions
_para_dynamic_complete() {
    local command_line="${COMP_WORDS[*]}"
    local current_word="${COMP_WORDS[COMP_CWORD]}"
    local previous_word=""
    
    if [[ $COMP_CWORD -gt 0 ]]; then
        previous_word="${COMP_WORDS[COMP_CWORD-1]}"
    fi
    
    local suggestions
    suggestions=$(para complete-command \
        --command-line "$command_line" \
        --current-word "$current_word" \
        --previous-word "$previous_word" \
        --position "$COMP_CWORD" 2>/dev/null || true)
    
    if [[ -n "$suggestions" ]]; then
        while IFS= read -r suggestion; do
            # Extract just the completion text (before any colon)
            local completion="${suggestion%%:*}"
            if [[ -n "$completion" ]]; then
                COMPREPLY+=("$completion")
            fi
        done <<< "$suggestions"
    fi
}

# Fallback completion functions
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

_para_complete_files() {
    COMPREPLY=($(compgen -f -- "$1"))
}

# Enhanced completion function
_para_completion() {
    local cur prev words cword
    _init_completion || return

    # Try dynamic completion first
    _para_dynamic_complete
    
    # If dynamic completion didn't provide anything, use fallbacks
    if [[ ${#COMPREPLY[@]} -eq 0 ]]; then
        case "${prev}" in
            resume|recover|cancel)
                if [[ "${prev}" == "recover" ]]; then
                    _para_complete_archived_sessions "${cur}"
                else
                    _para_complete_sessions "${cur}"
                fi
                return 0
                ;;
            --file|-f)
                _para_complete_files "${cur}"
                return 0
                ;;
            --branch)
                _para_complete_branches "${cur}"
                return 0
                ;;
        esac

        case "${words[1]}" in
            finish|integrate)
                if [[ $cword -eq 3 ]]; then
                    _para_complete_sessions "${cur}"
                fi
                ;;
        esac
    fi
}

# Register the enhanced completion
complete -F _para_completion para
"#.to_string()
    }

    fn generate_zsh_dynamic() -> String {
        r#"
# Para dynamic completion functions for zsh
_para_sessions() {
    local sessions
    sessions=(${(f)"$(para list --quiet 2>/dev/null | grep -o '^[a-zA-Z0-9_-]*' 2>/dev/null || true)"})
    _describe 'sessions' sessions
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
        _describe 'branches' branches
    fi
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
                finish|integrate)
                    if [[ $CURRENT -eq 3 ]]; then
                        _para_sessions
                    fi
                    ;;
                dispatch)
                    case $words[CURRENT-1] in
                        --file|-f)
                            _files
                            ;;
                        *)
                            if [[ $words[CURRENT] == --* ]]; then
                                _arguments '--file[Read prompt from file]:file:_files'
                            fi
                            ;;
                    esac
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
        'integrate:Squash commits and merge into base branch'
        'cancel:Cancel session (moves to archive)'
        'clean:Remove all active sessions'
        'list:List active sessions'
        'resume:Resume session in IDE'
        'recover:Recover cancelled session from archive'
        'continue:Complete merge after resolving conflicts'
        'config:Setup configuration'
        'completion:Generate shell completion script'
    )
    _describe 'commands' commands
}

# Register the completion
compdef _para para
"#.to_string()
    }

    fn generate_fish_dynamic() -> String {
        r#"
# Para completion for fish shell

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

function __para_needs_command
    set cmd (commandline -opc)
    if [ (count $cmd) -eq 1 ]
        return 0
    end
    return 1
end

function __para_using_command
    set cmd (commandline -opc)
    if [ (count $cmd) -gt 1 ]
        if [ $argv[1] = $cmd[2] ]
            return 0
        end
    end
    return 1
end

# Commands
complete -f -c para -n '__para_needs_command' -a 'start' -d 'Create session with optional name'
complete -f -c para -n '__para_needs_command' -a 'dispatch' -d 'Start Claude Code session with prompt'
complete -f -c para -n '__para_needs_command' -a 'finish' -d 'Squash all changes into single commit'
complete -f -c para -n '__para_needs_command' -a 'integrate' -d 'Squash commits and merge into base branch'
complete -f -c para -n '__para_needs_command' -a 'cancel' -d 'Cancel session (moves to archive)'
complete -f -c para -n '__para_needs_command' -a 'clean' -d 'Remove all active sessions'
complete -f -c para -n '__para_needs_command' -a 'list' -d 'List active sessions'
complete -f -c para -n '__para_needs_command' -a 'resume' -d 'Resume session in IDE'
complete -f -c para -n '__para_needs_command' -a 'recover' -d 'Recover cancelled session from archive'
complete -f -c para -n '__para_needs_command' -a 'continue' -d 'Complete merge after resolving conflicts'
complete -f -c para -n '__para_needs_command' -a 'config' -d 'Setup configuration'
complete -f -c para -n '__para_needs_command' -a 'completion' -d 'Generate shell completion script'

# Dynamic completions for specific commands
complete -f -c para -n '__para_using_command resume' -a '(__para_sessions)'
complete -f -c para -n '__para_using_command cancel' -a '(__para_sessions)'
complete -f -c para -n '__para_using_command recover' -a '(__para_archived_sessions)'

# File completions
complete -c para -n '__para_using_command dispatch' -s f -l file -F -d 'Read prompt from file'

# Branch completions
complete -f -c para -n '__para_using_command start' -l branch -a '(__para_branches)' -d 'Branch name'
complete -f -c para -n '__para_using_command finish' -l branch -a '(__para_branches)' -d 'Custom branch name after finishing'

# Session completions for finish/integrate third argument
complete -f -c para -n '__para_using_command finish; and test (count (commandline -opc)) -eq 4' -a '(__para_sessions)'
complete -f -c para -n '__para_using_command integrate; and test (count (commandline -opc)) -eq 4' -a '(__para_sessions)'
"#.to_string()
    }

    fn generate_powershell_dynamic() -> String {
        r#"
# Para completion for PowerShell

function Get-ParaSessions {
    try {
        $sessions = para list --quiet 2>$null | Where-Object { $_ -match '^[a-zA-Z0-9_-]*' }
        return $sessions
    } catch {
        return @()
    }
}

function Get-ParaArchivedSessions {
    try {
        $sessions = para list --archived --quiet 2>$null | Where-Object { $_ -match '^[a-zA-Z0-9_-]*' }
        return $sessions
    } catch {
        return @()
    }
}

function Get-GitBranches {
    try {
        if (git rev-parse --git-dir 2>$null) {
            $branches = git branch -a 2>$null | ForEach-Object { 
                $_.TrimStart('* ').Replace('remotes/origin/', '') 
            } | Where-Object { $_ -notmatch '^HEAD' } | Sort-Object -Unique
            return $branches
        }
    } catch {
        return @()
    }
}

Register-ArgumentCompleter -Native -CommandName para -ScriptBlock {
    param($commandName, $wordToComplete, $cursorPosition)
    
    $command = $wordToComplete
    $words = $command.Split(' ', [StringSplitOptions]::RemoveEmptyEntries)
    
    if ($words.Count -le 1) {
        # Complete main commands
        $commands = @(
            'start', 'dispatch', 'finish', 'integrate', 'cancel', 
            'clean', 'list', 'resume', 'recover', 'continue', 
            'config', 'completion'
        )
        $commands | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    } elseif ($words.Count -ge 2) {
        $subcommand = $words[1]
        
        switch ($subcommand) {
            'resume' {
                Get-ParaSessions | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', "Session: $_")
                }
            }
            'cancel' {
                Get-ParaSessions | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', "Session: $_")
                }
            }
            'recover' {
                Get-ParaArchivedSessions | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', "Archived session: $_")
                }
            }
            'finish' {
                if ($words.Count -eq 4) {
                    Get-ParaSessions | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', "Session: $_")
                    }
                }
            }
            'integrate' {
                if ($words.Count -eq 4) {
                    Get-ParaSessions | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', "Session: $_")
                    }
                }
            }
            'completion' {
                $shells = @('bash', 'zsh', 'fish', 'powershell')
                $shells | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', "Shell: $_")
                }
            }
        }
    }
}
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
            Shell::PowerShell => r#"# Installation instructions for PowerShell completion:

# Option 1: Add to your PowerShell profile
if (!(Test-Path -Path $PROFILE)) {
    New-Item -ItemType File -Path $PROFILE -Force
}
para completion powershell | Add-Content -Path $PROFILE

# Option 2: Install for all users (requires admin)
para completion powershell | Add-Content -Path $PROFILE.AllUsersAllHosts

# Then reload PowerShell or run:
. $PROFILE"#
                .to_string(),
        }
    }
}
