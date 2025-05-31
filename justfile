# Justfile for pursor project
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
    @echo "🚀 Installing pursor globally..."
    ./install-pursor.sh

# Run the current local pursor.sh
run *ARGS:
    @echo "🏃 Running local pursor.sh..."
    ./pursor.sh {{ARGS}}

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
test FILE="": install-dev
<<<<<<< HEAD
    @echo "🧪 Running all tests..."
=======
    @echo "🧪 Running tests..."
>>>>>>> e0e9dab (allow executing unit tests seperately)
    @if [ ! -d "tests" ]; then \
        echo "Creating tests directory..."; \
        mkdir -p tests; \
    fi
    @if [ "{{FILE}}" != "" ]; then \
        echo "Running specific test file: {{FILE}}"; \
        bats "{{FILE}}"; \
    else \
        echo "Running all tests..."; \
        echo "Running unit tests..."; \
        bats tests/test_pursor.bats || true; \
<<<<<<< HEAD
        bats tests/test_pursor_units.bats || true; \
=======
>>>>>>> e0e9dab (allow executing unit tests seperately)
        echo "Running integration tests..."; \
        bats tests/test_pursor_integration.bats || true; \
    fi

# Run only integration tests
test-integration: install-dev
    @echo "🧪 Running integration tests..."
    @bats tests/test_pursor_integration.bats

# Run only unit tests  
test-unit: install-dev
    @echo "🧪 Running unit tests..."
    @bats tests/test_pursor.bats
    @bats tests/test_pursor_units.bats

# Run linting with shellcheck and shfmt
lint: install-dev
    @echo "🔍 Running linting checks..."
    @echo "Running shellcheck..."
    shellcheck -e {{SHELLCHECK_DISABLE}} pursor.sh install-pursor.sh lib/*.sh || true
    @echo "Running shfmt check..."
    shfmt -d -i 2 pursor.sh install-pursor.sh lib/*.sh || true

# Fix formatting with shfmt
fmt: install-dev
    @echo "🎨 Fixing shell script formatting..."
    shfmt -w -i 2 pursor.sh install-pursor.sh lib/*.sh

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