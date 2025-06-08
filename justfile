# Justfile for para project
# https://github.com/casey/just

# Set shell to use for command execution
set shell := ["bash", "-c"]

# Default recipe (when just is run without arguments)
default:
    @just --list

# Variables
BATS_VERSION := "v1.10.0"
SHELLCHECK_DISABLE := "SC1091,SC2086"

# Install the project globally
install:
    @echo "🚀 Installing para globally..."
    ./install-para.sh

# Uninstall para globally  
uninstall:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "🗑️  Uninstalling para..."
    
    # Define paths
    INSTALL_BIN_DIR="$HOME/.local/bin"
    INSTALL_BASE_DIR="$HOME/.local/lib/para"
    TEMPLATE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/para"
    PARA_BIN="$INSTALL_BIN_DIR/para"
    
    # Remove the binary
    if [ -f "$PARA_BIN" ]; then
        echo "🗑️  Removing para binary: $PARA_BIN"
        rm -f "$PARA_BIN"
    else
        echo "ℹ️  Para binary not found at $PARA_BIN"
    fi
    
    # Remove the installation directory
    if [ -d "$INSTALL_BASE_DIR" ]; then
        echo "🗑️  Removing para installation: $INSTALL_BASE_DIR"
        rm -rf "$INSTALL_BASE_DIR"
    else
        echo "ℹ️  Para installation directory not found at $INSTALL_BASE_DIR"
    fi
    
    # Remove the template and user data directories
    if [ -d "$TEMPLATE_DIR" ]; then
        echo "🗑️  Removing para data directory: $TEMPLATE_DIR"
        echo "   (includes template and global user data)"
        rm -rf "$TEMPLATE_DIR"
    else
        echo "ℹ️  Para data directory not found at $TEMPLATE_DIR"
    fi
    
    # Check for PATH entries in shell configs (informational only)
    echo "⚠️  Note: You may want to manually remove para PATH entries from your shell config:"
    echo "   - ~/.bashrc (bash)"
    echo "   - ~/.zshrc (zsh)" 
    echo "   - ~/.config/fish/config.fish (fish)"
    echo "   - ~/.profile (generic)"
    echo "   Look for lines containing '$INSTALL_BIN_DIR'"
    
    echo "✅ Para uninstalled successfully!"

# Run the current local para.sh
run *ARGS:
    @echo "🏃 Running local para.sh..."
    ./para.sh {{ARGS}}

# Install development dependencies
install-dev:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "📦 Installing development dependencies..."
    
    # Install bats-core for testing
    if ! command -v bats &> /dev/null; then
        echo "Installing bats-core..."
        if [[ "$OSTYPE" == "darwin"* ]]; then
            if command -v brew &> /dev/null; then
                brew install bats-core
            else
                echo "Please install Homebrew to automatically install bats-core"
                echo "Or install bats-core manually: https://github.com/bats-core/bats-core"
                exit 1
            fi
        elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
            # Try package managers for Linux
            if command -v apt-get &> /dev/null; then
                sudo apt-get update && sudo apt-get install -y bats
            elif command -v yum &> /dev/null; then
                sudo yum install -y bats
            elif command -v dnf &> /dev/null; then
                sudo dnf install -y bats
            else
                echo "Please install bats-core manually: https://github.com/bats-core/bats-core"
                exit 1
            fi
        else
            echo "Please install bats-core manually: https://github.com/bats-core/bats-core"
            exit 1
        fi
    else
        echo "✅ bats-core already installed"
    fi
    
    # Install shellcheck
    if ! command -v shellcheck &> /dev/null; then
        echo "Installing shellcheck..."
        if [[ "$OSTYPE" == "darwin"* ]]; then
            if command -v brew &> /dev/null; then
                brew install shellcheck
            else
                echo "Please install Homebrew to automatically install shellcheck"
                exit 1
            fi
        elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
            if command -v apt-get &> /dev/null; then
                sudo apt-get update && sudo apt-get install -y shellcheck
            elif command -v yum &> /dev/null; then
                sudo yum install -y ShellCheck
            elif command -v dnf &> /dev/null; then
                sudo dnf install -y ShellCheck
            else
                echo "Please install shellcheck manually"
                exit 1
            fi
        else
            echo "Please install shellcheck manually"
            exit 1
        fi
    else
        echo "✅ shellcheck already installed"
    fi
    
    # Install shfmt
    if ! command -v shfmt &> /dev/null; then
        echo "Installing shfmt..."
        if [[ "$OSTYPE" == "darwin"* ]]; then
            if command -v brew &> /dev/null; then
                brew install shfmt
            else
                echo "Please install Homebrew to automatically install shfmt"
                exit 1
            fi
        elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
            # Install shfmt via go or download binary
            if command -v go &> /dev/null; then
                go install mvdan.cc/sh/v3/cmd/shfmt@latest
            else
                echo "Installing shfmt binary..."
                curl -L -o /tmp/shfmt https://github.com/mvdan/sh/releases/latest/download/shfmt_v3.7.0_linux_amd64
                chmod +x /tmp/shfmt
                sudo mv /tmp/shfmt /usr/local/bin/shfmt
            fi
        else
            echo "Please install shfmt manually: https://github.com/mvdan/sh"
            exit 1
        fi
    else
        echo "✅ shfmt already installed"
    fi
    
    echo "✅ Development dependencies installed"

# Run tests using bats
test FILE="": install-dev lint
    @echo "🧪 Running tests..."
    @if [ ! -d "tests" ]; then \
        echo "Creating tests directory..."; \
        mkdir -p tests; \
    fi
    @if [ "{{FILE}}" != "" ]; then \
        echo "Running specific test file: {{FILE}}"; \
        bats "{{FILE}}"; \
    else \
        echo "Running all tests (99 total)..."; \
        echo "Running unit tests..."; \
        bats tests/test_para_units.bats || true; \
        echo "Running prompt feature tests..."; \
        bats tests/test_para_prompt_features.bats || true; \
        echo "Running argument parsing tests..."; \
        bats tests/test_para_argument_parsing.bats || true; \
        echo "Running friendly names tests..."; \
        bats tests/test_friendly_names.bats || true; \
        echo "Running integration tests..."; \
        bats tests/test_para_integration.bats || true; \
    fi

# Run only integration tests
test-integration: install-dev
    @echo "🧪 Running integration tests..."
    @bats tests/test_para_integration.bats

# Run only unit tests  
test-unit: install-dev
    @echo "🧪 Running unit tests..."
    @bats tests/test_para_units.bats

# Run only friendly names tests
test-friendly: install-dev
    @echo "🧪 Running friendly names tests..."
    @bats tests/test_friendly_names.bats

# Run only prompt feature tests
test-prompts: install-dev
    @echo "🧪 Running prompt feature tests..."
    @bats tests/test_para_prompt_features.bats

# Run only argument parsing tests  
test-args: install-dev
    @echo "🧪 Running argument parsing tests..."
    @bats tests/test_para_argument_parsing.bats

# Run performance benchmarks
benchmark:
    #!/usr/bin/env bash
    set -e
    
    echo "⚡ Running para performance benchmarks..."
    
    # Check if we're in a git repository
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        echo "❌ Not in a Git repository. Performance benchmarks require a Git repository."
        echo "💡 Navigate to a Git repository or run: git init"
        exit 1
    fi
    
    # Check if benchmark script exists and is executable
    if [ ! -f "scripts/benchmark-performance.sh" ]; then
        echo "❌ Benchmark script not found at scripts/benchmark-performance.sh"
        exit 1
    fi
    
    if [ ! -x "scripts/benchmark-performance.sh" ]; then
        echo "🔧 Making benchmark script executable..."
        chmod +x scripts/benchmark-performance.sh
    fi
    
    # Run the benchmark
    ./scripts/benchmark-performance.sh

# Run linting with shellcheck and shfmt
lint: install-dev
    @echo "🔍 Running linting checks..."
    @echo "Running shellcheck..."
    shellcheck -e {{SHELLCHECK_DISABLE}} para.sh install-para.sh lib/*.sh || true
    @echo "Running shfmt check..."
    shfmt -d -i 2 para.sh install-para.sh lib/*.sh || true

# Fix formatting with shfmt
fmt: install-dev
    @echo "🎨 Fixing shell script formatting..."
    shfmt -w -i 2 para.sh install-para.sh lib/*.sh

# Setup git hooks
setup-hooks:
    @echo "🪝 Setting up git hooks..."
    @mkdir -p .git/hooks
    @cp scripts/pre-commit .git/hooks/pre-commit
    @chmod +x .git/hooks/pre-commit
    @cp scripts/pre-push .git/hooks/pre-push
    @chmod +x .git/hooks/pre-push
    @echo "✅ Git hooks configured:"
    @echo "   • pre-commit: runs tests"
    @echo "   • pre-push: runs linting"

# Clean up development artifacts
clean:
    @echo "🧹 Cleaning up..."
    @rm -rf tests/test_*.tmp
    @rm -rf .bats_tmp*
    @echo "✅ Cleaned up development artifacts"

# Show project status
status:
    @echo "📊 Project Status"
    @echo "=================="
    @echo "Shell scripts:"
    @find . -name "*.sh" -not -path "./subtrees/*" -not -path "./.git/*" | wc -l | sed 's/^/  /'
    @echo ""
    @echo "Dependencies:"
    @command -v bats >/dev/null 2>&1 && echo "  ✅ bats" || echo "  ❌ bats"
    @command -v shellcheck >/dev/null 2>&1 && echo "  ✅ shellcheck" || echo "  ❌ shellcheck"
    @command -v shfmt >/dev/null 2>&1 && echo "  ✅ shfmt" || echo "  ❌ shfmt"
    @echo ""
    @echo "Git hooks:"
    @[ -f .git/hooks/pre-commit ] && echo "  ✅ pre-commit" || echo "  ❌ pre-commit"
    @[ -f .git/hooks/pre-push ] && echo "  ✅ pre-push" || echo "  ❌ pre-push"

# Development workflow: install deps, setup hooks, run tests and lint
dev-setup: install-dev setup-hooks test lint
    @echo "🎉 Development environment ready!"
    @echo ""
    @echo "💡 Available development commands:"
    @echo "   just test           - Run all tests"
    @echo "   just lint           - Run linting checks"
    @echo "   just fmt            - Fix formatting"
    @echo "   just benchmark      - Run performance benchmarks"
    @echo "   just status         - Show project status"

# Create a release - triggers GitHub Actions to build and publish
release BUMP="patch":
    #!/usr/bin/env bash
    set -e
    
    # Check if we're on master branch
    current_branch=$(git branch --show-current)
    if [ "$current_branch" != "master" ]; then
        echo "Error: Must be on master branch to create a release"
        exit 1
    fi
    
    # Pull latest changes and check no staged changes exist
    git pull origin master
    if [ -n "$(git diff --cached --name-only)" ]; then
        echo "Error: Staged changes detected. Commit or unstage changes first."
        exit 1
    fi
    
    # Get the latest version tag
    latest_tag=$(git tag -l "v*" | sort -V | tail -1)
    
    if [ -z "$latest_tag" ]; then
        # No existing tags, start with v1.0.0
        new_version="v1.0.0"
        echo "No existing version tags found. Starting with $new_version"
    else
        echo "Latest version: $latest_tag"
        
        # Remove 'v' prefix and split version into parts
        current_version=${latest_tag#v}
        IFS='.' read -r major minor patch <<< "$current_version"
        
        # Increment based on bump type
        case "{{BUMP}}" in
            major)
                major=$((major + 1))
                minor=0
                patch=0
                ;;
            minor)
                minor=$((minor + 1))
                patch=0
                ;;
            patch)
                patch=$((patch + 1))
                ;;
            *)
                echo "Error: Invalid bump type '{{BUMP}}'. Use 'major', 'minor', or 'patch'"
                exit 1
                ;;
        esac
        
        new_version="v$major.$minor.$patch"
        echo "Bumping {{BUMP}} version: $latest_tag → $new_version"
    fi
    
    # Create and push tag to trigger release workflow
    echo "Creating release tag: $new_version"
    git tag "$new_version"
    git push origin "$new_version"
    
    echo "✅ Release $new_version triggered! Monitor at: https://github.com/2mawi2/para/actions" 