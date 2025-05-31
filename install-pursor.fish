#!/usr/bin/env fish

# Fish installation script for pursor
# This script installs pursor globally so you can call it from anywhere

set -l script_dir (dirname (status -f))
set -l pursor_script "$script_dir/pursor.sh"
set -l install_dir "$HOME/.local/bin"
set -l pursor_bin "$install_dir/pursor"
set -l fish_config "$HOME/.config/fish/config.fish"

echo "🚀 Installing pursor globally..."

# Check if pursor.sh exists
if not test -f $pursor_script
    echo "❌ Error: pursor.sh not found in $script_dir"
    echo "   Make sure you're running this script from the same directory as pursor.sh"
    exit 1
end

# Create ~/.local/bin if it doesn't exist
if not test -d $install_dir
    echo "📁 Creating $install_dir"
    mkdir -p $install_dir
end

# Copy pursor.sh to ~/.local/bin/pursor
echo "📋 Copying pursor.sh to $pursor_bin"
cp $pursor_script $pursor_bin

# Make it executable
echo "🔧 Making pursor executable"
chmod +x $pursor_bin

# Check if ~/.local/bin is in PATH
if not contains $install_dir $fish_user_paths
    echo "🛣️  Adding $install_dir to fish PATH"
    fish_add_path $install_dir
else
    echo "✅ $install_dir already in PATH"
end

# Verify installation
if test -x $pursor_bin
    echo ""
    echo "✅ pursor installed successfully!"
    echo "   Location: $pursor_bin"
    echo "   You can now run 'pursor' from anywhere"
    echo ""
    echo "🧪 Testing installation..."
    
    # Test that pursor is available in PATH
    if command -v pursor >/dev/null 2>&1
        echo "✅ pursor command is available in PATH"
        echo ""
        echo "🎉 Installation complete! Try running:"
        echo "   pursor --help"
    else
        echo "⚠️  pursor command not found in PATH"
        echo "   You may need to restart your shell or run:"
        echo "   source $fish_config"
    end
else
    echo "❌ Installation failed - pursor binary not executable"
    exit 1
end

echo ""
echo "📖 Quick usage reminder:"
echo "   pursor                    # create new session & open Cursor"
echo "   pursor merge \"message\"    # merge current session"
echo "   pursor list               # list all active sessions"
echo "   pursor continue           # continue merge after conflicts"
echo "   pursor cancel             # cancel current session"
echo "   pursor clean              # clean up all sessions" 