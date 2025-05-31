#!/usr/bin/env sh

# Universal shell installation script for pursor
# This script installs pursor globally and works with bash, zsh, fish, and other POSIX shells

# Get script directory in a POSIX-compliant way
script_dir="$(dirname "$0")"
case "$script_dir" in
    /*) ;;
    *) script_dir="$PWD/$script_dir" ;;
esac

pursor_script="$script_dir/pursor.sh"
install_dir="$HOME/.local/bin"
pursor_bin="$install_dir/pursor"

echo "🚀 Installing pursor globally..."

# Check if pursor.sh exists
if [ ! -f "$pursor_script" ]; then
    echo "❌ Error: pursor.sh not found in $script_dir"
    echo "   Make sure you're running this script from the same directory as pursor.sh"
    exit 1
fi

# Create ~/.local/bin if it doesn't exist
if [ ! -d "$install_dir" ]; then
    echo "📁 Creating $install_dir"
    mkdir -p "$install_dir"
fi

# Copy pursor.sh to ~/.local/bin/pursor
echo "📋 Copying pursor.sh to $pursor_bin"
cp "$pursor_script" "$pursor_bin"

# Make it executable
echo "🔧 Making pursor executable"
chmod +x "$pursor_bin"

# Auto-detect current shell and set appropriate config file
detect_shell_config() {
    # Check if we're running in a specific shell
    if [ -n "$FISH_VERSION" ]; then
        echo "fish"
        return
    elif [ -n "$ZSH_VERSION" ]; then
        echo "zsh"
        return
    elif [ -n "$BASH_VERSION" ]; then
        echo "bash"
        return
    fi
    
    # Fallback: check the SHELL environment variable
    case "$SHELL" in
        */fish) echo "fish" ;;
        */zsh) echo "zsh" ;;
        */bash) echo "bash" ;;
        *) echo "unknown" ;;
    esac
}

shell_type=$(detect_shell_config)

# Check if ~/.local/bin is in PATH
path_contains_install_dir() {
    case ":$PATH:" in
        *":$install_dir:"*) return 0 ;;
        *) return 1 ;;
    esac
}

if ! path_contains_install_dir; then
    echo "🛣️  Adding $install_dir to PATH for $shell_type shell"
    
    case "$shell_type" in
        fish)
            fish_config="$HOME/.config/fish/config.fish"
            if command -v fish >/dev/null 2>&1; then
                # Use fish's built-in fish_add_path if available
                if fish -c "fish_add_path $install_dir" 2>/dev/null; then
                    echo "✅ Added $install_dir to fish PATH using fish_add_path"
                else
                    # Fallback to manual config modification
                    mkdir -p "$(dirname "$fish_config")"
                    echo "" >> "$fish_config"
                    echo "# Added by pursor installer" >> "$fish_config"
                    echo "set -gx PATH \$PATH $install_dir" >> "$fish_config"
                    echo "✅ Added $install_dir to fish PATH in $fish_config"
                fi
            else
                echo "⚠️  Fish shell not found, but detected fish environment"
            fi
            ;;
        zsh)
            zsh_config="$HOME/.zshrc"
            echo "" >> "$zsh_config"
            echo "# Added by pursor installer" >> "$zsh_config"
            echo "export PATH=\"\$PATH:$install_dir\"" >> "$zsh_config"
            echo "✅ Added $install_dir to zsh PATH in $zsh_config"
            ;;
        bash)
            bash_config="$HOME/.bashrc"
            echo "" >> "$bash_config"
            echo "# Added by pursor installer" >> "$bash_config"
            echo "export PATH=\"\$PATH:$install_dir\"" >> "$bash_config"
            echo "✅ Added $install_dir to bash PATH in $bash_config"
            ;;
        *)
            # Generic POSIX shell - try common config files
            if [ -f "$HOME/.profile" ]; then
                profile_config="$HOME/.profile"
            else
                profile_config="$HOME/.profile"
                touch "$profile_config"
            fi
            echo "" >> "$profile_config"
            echo "# Added by pursor installer" >> "$profile_config"
            echo "export PATH=\"\$PATH:$install_dir\"" >> "$profile_config"
            echo "✅ Added $install_dir to PATH in $profile_config (generic shell)"
            ;;
    esac
    
    # Also add to current session
    export PATH="$PATH:$install_dir"
    echo "   (restart shell or source config file to persist)"
else
    echo "✅ $install_dir already in PATH"
fi

# Verify installation
if [ -x "$pursor_bin" ]; then
    echo ""
    echo "✅ pursor installed successfully!"
    echo "   Location: $pursor_bin"
    echo "   Shell: $shell_type"
    echo "   You can now run 'pursor' from anywhere"
    echo ""
    echo "🧪 Testing installation..."
    
    # Test that pursor is available in PATH
    if command -v pursor >/dev/null 2>&1; then
        echo "✅ pursor command is available in PATH"
        echo ""
        echo "🎉 Installation complete! Try running:"
        echo "   pursor --help"
    else
        echo "⚠️  pursor command not found in PATH"
        echo "   You may need to restart your shell or source your config file"
    fi
else
    echo "❌ Installation failed - pursor binary not executable"
    exit 1
fi

echo ""
echo "📖 Quick usage reminder:"
echo "   pursor                    # create new session & open Cursor"
echo "   pursor merge \"message\"    # merge current session"
echo "   pursor list               # list all active sessions"
echo "   pursor continue           # continue merge after conflicts"
echo "   pursor cancel             # cancel current session"
echo "   pursor clean              # clean up all sessions" 