#!/usr/bin/env sh

# Universal shell installation script for para
# This script installs para globally and works with bash, zsh, fish, and other POSIX shells

# Get script directory in a POSIX-compliant way
script_dir="$(dirname "$0")"
case "$script_dir" in
/*) ;;
*) script_dir="$PWD/$script_dir" ;;
esac

para_script="$script_dir/para.sh"
para_lib_dir="$script_dir/lib"
install_base_dir="$HOME/.local/lib/para"
install_bin_dir="$HOME/.local/bin"
para_bin="$install_bin_dir/para"

echo "🚀 Installing para globally..."

# Check if para.sh exists
if [ ! -f "$para_script" ]; then
  echo "❌ Error: para.sh not found in $script_dir"
  echo "   Make sure you're running this script from the same directory as para.sh"
  exit 1
fi

# Check if lib directory exists
if [ ! -d "$para_lib_dir" ]; then
  echo "❌ Error: lib/ directory not found in $script_dir"
  echo "   The modular para installation requires the lib/ directory with supporting modules"
  exit 1
fi

# Create directories
echo "📁 Creating installation directories"
mkdir -p "$install_base_dir"
mkdir -p "$install_bin_dir"

# Copy the entire para structure to ~/.local/lib/para/
echo "📋 Installing para modules to $install_base_dir"
cp "$para_script" "$install_base_dir/"
cp -r "$para_lib_dir" "$install_base_dir/"

# Make the main script executable
chmod +x "$install_base_dir/para.sh"

# Create wrapper script in ~/.local/bin/para
echo "🔧 Creating para command wrapper"
cat >"$para_bin" <<'EOF'
#!/usr/bin/env sh
# Para wrapper script - calls the main para installation

# Find the installation directory
PARA_INSTALL_DIR="$HOME/.local/lib/para"

if [ ! -f "$PARA_INSTALL_DIR/para.sh" ]; then
    echo "❌ Error: para installation not found at $PARA_INSTALL_DIR" >&2
    echo "   Please reinstall para or check your installation" >&2
    exit 1
fi

# Execute the main para script with all arguments
exec "$PARA_INSTALL_DIR/para.sh" "$@"
EOF

# Make wrapper executable
chmod +x "$para_bin"

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
  *":$install_bin_dir:"*) return 0 ;;
  *) return 1 ;;
  esac
}

if ! path_contains_install_dir; then
  echo "🛣️  Adding $install_bin_dir to PATH for $shell_type shell"

  case "$shell_type" in
  fish)
    fish_config="$HOME/.config/fish/config.fish"
    if command -v fish >/dev/null 2>&1; then
      # Use fish's built-in fish_add_path if available
      if fish -c "fish_add_path $install_bin_dir" 2>/dev/null; then
        echo "✅ Added $install_bin_dir to fish PATH using fish_add_path"
      else
        # Fallback to manual config modification
        mkdir -p "$(dirname "$fish_config")"
        {
          echo ""
          echo "# Added by para installer"
          echo "set -gx PATH \$PATH $install_bin_dir"
        } >>"$fish_config"
        echo "✅ Added $install_bin_dir to fish PATH in $fish_config"
      fi
    else
      echo "⚠️  Fish shell not found, but detected fish environment"
    fi
    ;;
  zsh)
    zsh_config="$HOME/.zshrc"
    echo "" >>"$zsh_config"
    echo "# Added by para installer" >>"$zsh_config"
    echo "export PATH=\"\$PATH:$install_bin_dir\"" >>"$zsh_config"
    echo "✅ Added $install_bin_dir to zsh PATH in $zsh_config"
    ;;
  bash)
    bash_config="$HOME/.bashrc"
    echo "" >>"$bash_config"
    echo "# Added by para installer" >>"$bash_config"
    echo "export PATH=\"\$PATH:$install_bin_dir\"" >>"$bash_config"
    echo "✅ Added $install_bin_dir to bash PATH in $bash_config"
    ;;
  *)
    # Generic POSIX shell - try common config files
    if [ -f "$HOME/.profile" ]; then
      profile_config="$HOME/.profile"
    else
      profile_config="$HOME/.profile"
      touch "$profile_config"
    fi
    echo "" >>"$profile_config"
    echo "# Added by para installer" >>"$profile_config"
    echo "export PATH=\"\$PATH:$install_bin_dir\"" >>"$profile_config"
    echo "✅ Added $install_bin_dir to PATH in $profile_config (generic shell)"
    ;;
  esac

  # Also add to current session
  export PATH="$PATH:$install_bin_dir"
  echo "   (restart shell or source config file to persist)"
else
  echo "✅ $install_bin_dir already in PATH"
fi

# Verify installation
if [ -x "$para_bin" ] && [ -f "$install_base_dir/para.sh" ]; then
  echo ""
  echo "✅ para installed successfully!"
  echo "   Installation: $install_base_dir"
  echo "   Wrapper: $para_bin"
  echo "   Shell: $shell_type"
  echo "   You can now run 'para' from anywhere"
  echo ""
  echo "🧪 Testing installation..."

  # Test that para is available in PATH
  if command -v para >/dev/null 2>&1; then
    echo "✅ para command is available in PATH"
    echo ""
    echo "🎉 Installation complete! Try running:"
    echo "   para start                  # create new session"
    echo "   para start feature-auth     # create named session"
    echo "   para list                   # list active sessions"
    echo "   para finish \"message\"       # finish current session"
    echo "   para cancel                 # cancel current session"
    echo "   para continue               # continue finish after conflicts"
    echo "   para clean                  # clean all sessions"
  else
    echo "⚠️  para command not found in PATH"
    echo "   You may need to restart your shell or source your config file"
  fi
else
  echo "❌ Installation failed - para installation incomplete"
  exit 1
fi

echo ""
echo "🎯 USAGE:"
echo "   para start                  # create new session"
echo "   para start feature-auth     # create named session"
echo "   para list                   # list active sessions"
echo "   para finish \"message\"       # finish current session"
echo "   para cancel                 # cancel current session"
echo "   para continue               # continue finish after conflicts"
echo "   para clean                  # clean all sessions"
echo ""

echo ""
echo "📁 Installation structure:"
echo "   Main installation: $install_base_dir"
echo "   ├── para.sh           # Main script"
echo "   └── lib/                # Library modules"
echo "       ├── para-config.sh   # Configuration management"
echo "       ├── para-utils.sh    # Utility functions"
echo "       ├── para-git.sh      # Git operations"
echo "       ├── para-session.sh  # Session management"
echo "       └── para-ide.sh      # IDE integration"
echo "   Command wrapper: $para_bin"
