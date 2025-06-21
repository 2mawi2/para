# Justfile for Para - Rust implementation
# https://github.com/casey/just

# Set shell to use for command execution
set shell := ["bash", "-c"]

# Default recipe (when just is run without arguments)
default:
    @just --list

# Build debug binary
build:
    cargo build

# Build optimized release binary
build-release:
    cargo build --release

# Build TypeScript MCP server
build-ts:
    @echo "📦 Building TypeScript MCP server..."
    @cd mcp-server-ts && npm install && npm run build

# Install para CLI and MCP server
install: build-release build-ts
    @echo "🚀 Installing Para binaries..."
    @mkdir -p ~/.local/bin
    @cp target/release/para ~/.local/bin/para
    @echo "✅ Para CLI binary installed to ~/.local/bin/para"
    @# Check if MCP server exists before copying
    @if [ -f "mcp-server-ts/build/para-mcp-server-new.js" ]; then \
        echo "📦 Installing MCP server..."; \
        cp mcp-server-ts/build/para-mcp-server-new.js ~/.local/bin/para-mcp-server; \
        chmod +x ~/.local/bin/para-mcp-server; \
        echo "✅ Para MCP server installed to ~/.local/bin/para-mcp-server"; \
    else \
        echo "⚠️  MCP server not found. Run 'just build-ts' to build it"; \
    fi
    @echo ""
    @echo "🔧 Next steps:"
    @echo "   Run 'para mcp init' to configure MCP integration for your IDE"

# Uninstall para globally  
uninstall:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "🗑️  Uninstalling para..."
    
    # Define paths
    INSTALL_BIN_DIR="$HOME/.local/bin"
    PARA_BIN="$INSTALL_BIN_DIR/para"
    PARA_MCP_BIN="$INSTALL_BIN_DIR/para-mcp-server"
    
    # Remove the binaries
    if [ -f "$PARA_BIN" ]; then
        echo "🗑️  Removing para CLI binary: $PARA_BIN"
        rm -f "$PARA_BIN"
    else
        echo "ℹ️  Para CLI binary not found at $PARA_BIN"
    fi
    
    if [ -f "$PARA_MCP_BIN" ]; then
        echo "🗑️  Removing para MCP server: $PARA_MCP_BIN"
        rm -f "$PARA_MCP_BIN"
    else
        echo "ℹ️  Para MCP server not found at $PARA_MCP_BIN"
    fi
    
    echo "✅ Para uninstalled successfully!"

# Legacy MCP setup (use 'para mcp init' instead)
mcp-setup: install
    @echo "⚠️  'just mcp-setup' is deprecated"
    @echo "💡 Use 'para mcp init' instead for simplified setup"
    @echo ""
    @echo "Example usage:"
    @echo "  para mcp init                # Interactive setup"
    @echo "  para mcp init --claude-code  # Setup for Claude Code"
    @echo "  para mcp init --cursor       # Setup for Cursor"
    @echo "  para mcp init --vscode       # Setup for VS Code"

# Run comprehensive tests (Rust + TypeScript: formatting + tests + linting)
test *FILTER:
    #!/bin/bash
    set -euo pipefail
    
    # Check if filter is provided
    if [ "{{FILTER}}" != "" ]; then
        echo "🧪 Running Rust tests for: {{FILTER}}"
        cargo test {{FILTER}}
        exit 0
    fi
    
    echo "🧪 Running all checks (Rust + TypeScript)..."
    
    # Rust Format first
    printf "   Rust Format: "
    if cargo fmt --all --quiet 2>/dev/null; then
        echo "✅ formatted"
    else
        echo "❌ formatting failed"
        exit 1
    fi
    
    # Rust Tests
    printf "   Rust Tests: "
    if test_output=$(cargo test --message-format=short 2>&1); then
        if echo "$test_output" | grep -q "test result:"; then
            summary=$(echo "$test_output" | grep "test result:" | tail -1 | sed 's/test result: //')
            echo "✅ $summary"
        else
            echo "✅ passed"
        fi
    else
        echo "❌ FAILED"
        echo "$test_output" | grep -E "(test .* \.\.\. FAILED|assertion.*failed|panicked at)" | head -25
        exit 1
    fi
    
    # TypeScript Tests
    printf "   TypeScript Tests: "
    if [ -d "mcp-server-ts" ] && [ -f "mcp-server-ts/package.json" ]; then
        cd mcp-server-ts
        
        # Check if node_modules exists, install if needed
        if [ ! -d "node_modules" ] || [ ! -f "node_modules/.package-lock.json" ]; then
            printf "\n      Installing dependencies... "
            if npm install --silent >/dev/null 2>&1; then
                printf "✅\n      "
            else
                echo "❌ Failed to install dependencies"
                exit 1
            fi
        fi
        
        if ts_test_output=$(npm test 2>&1); then
            # Extract test summary from Jest output
            if echo "$ts_test_output" | grep -q "Tests:"; then
                summary=$(echo "$ts_test_output" | grep "Tests:" | tail -1 | sed 's/Tests:[[:space:]]*//')
                echo "✅ $summary"
            else
                echo "✅ passed"
            fi
        else
            echo "❌ FAILED"
            echo "$ts_test_output" | grep -E "(FAIL|Error:|✕)" | head -10
            exit 1
        fi
        cd ..
    else
        echo "⚠️  skipped (mcp-server-ts not found)"
    fi
    
    # Rust Clippy linting
    printf "   Rust Linting: "
    if cargo clippy --all-targets --all-features -- -W clippy::all >/dev/null 2>&1; then
        echo "✅ clean"
    else
        echo "❌ FAILED"
        cargo clippy --all-targets --all-features -- -W clippy::all
        exit 1
    fi
    
    echo "🎉 All checks passed (Rust + TypeScript)!"

# Run only Rust tests (no formatting/linting)
test-only *FILTER:
    #!/bin/bash
    if [ "{{FILTER}}" != "" ]; then
        cargo test {{FILTER}}
    else
        cargo test
    fi

# Run Rust linting with clippy
lint:
    @echo "🔍 Running Rust linting checks..."
    cargo clippy --all-targets --all-features -- -W clippy::all

# Format Rust code
fmt:
    @echo "🎨 Formatting Rust code..."
    cargo fmt --all

# Check Rust formatting
fmt-check:
    @echo "🔍 Checking Rust formatting..."
    cargo fmt --all -- --check

# Run the para binary with arguments
run *ARGS: build
    ./target/debug/para {{ARGS}}

# Setup git hooks for Rust development
setup-hooks:
    @echo "🪝 Setting up git hooks for Rust development..."
    @mkdir -p .git/hooks
    @echo '#!/bin/bash\nset -e\njust test' > .git/hooks/pre-commit
    @echo '#!/bin/bash\nset -e\njust lint' > .git/hooks/pre-push
    @chmod +x .git/hooks/pre-commit .git/hooks/pre-push
    @echo "✅ Git hooks configured:"
    @echo "   • pre-commit: runs comprehensive tests"
    @echo "   • pre-push: runs linting"

# Clean up Rust build artifacts
clean:
    @echo "🧹 Cleaning up Rust build artifacts..."
    cargo clean
    @echo "✅ Cleaned up development artifacts"

# Show project status
status:
    @echo "📊 Para Project Status"
    @echo "======================"
    @echo "Rust toolchain:"
    @rustc --version || echo "  ❌ rustc not found"
    @cargo --version || echo "  ❌ cargo not found"
    @echo ""
    @echo "Development tools:"
    @command -v git >/dev/null 2>&1 && echo "  ✅ git" || echo "  ❌ git"
    @command -v just >/dev/null 2>&1 && echo "  ✅ just" || echo "  ❌ just"
    @command -v npm >/dev/null 2>&1 && echo "  ✅ npm" || echo "  ❌ npm"
    @echo ""
    @echo "Git hooks:"
    @[ -f .git/hooks/pre-commit ] && echo "  ✅ pre-commit" || echo "  ❌ pre-commit"
    @[ -f .git/hooks/pre-push ] && echo "  ✅ pre-push" || echo "  ❌ pre-push"
    @echo ""
    @echo "Binary status:"
    @[ -f target/debug/para ] && echo "  ✅ debug CLI binary built" || echo "  ❌ debug CLI binary not found"
    @[ -f target/release/para ] && echo "  ✅ release CLI binary built" || echo "  ❌ release CLI binary not found"
    @[ -f mcp-server-ts/build/para-mcp-server-new.js ] && echo "  ✅ MCP server built" || echo "  ❌ MCP server not found (run: just build-ts)"
    @echo ""
    @echo "TypeScript status:"
    @[ -d mcp-server-ts/node_modules ] && echo "  ✅ dependencies installed" || echo "  ❌ dependencies not installed (run: just build-ts)"

# Development workflow setup
dev-setup: setup-hooks test
    @echo "🎉 Rust development environment ready!"
    @echo ""
    @echo "💡 Available development commands:"
    @echo "   just build          - Build debug binary"
    @echo "   just build-release  - Build release binary"
    @echo "   just build-ts       - Build TypeScript MCP server"
    @echo "   just test           - Run comprehensive tests (Rust + TypeScript)"
    @echo "   just test [filter]  - Run specific tests"
    @echo "   just lint           - Run clippy linting"
    @echo "   just fmt            - Format code"
    @echo "   just run [args]     - Run para with arguments"
    @echo "   just install        - Install para globally"
    @echo "   para mcp init       - Setup MCP integration (after install)"

# Create a release - triggers GitHub Actions to build and publish
release BUMP="patch":
    #!/usr/bin/env bash
    set -e
    
    # Check if we're on main branch
    current_branch=$(git branch --show-current)
    if [ "$current_branch" != "main" ]; then
        echo "Error: Must be on main branch to create a release"
        exit 1
    fi
    
    # Ensure tests pass before release
    echo "🧪 Running tests before release..."
    just test
    
    # Pull latest changes and check no staged changes exist
    git pull origin main
    if [ -n "$(git diff --cached --name-only)" ]; then
        echo "Error: Staged changes detected. Commit or unstage changes first."
        exit 1
    fi
    
    # Get current version from Cargo.toml
    current_version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    echo "Current version: $current_version"
    
    # Split version into parts
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
    
    new_version="$major.$minor.$patch"
    echo "Bumping {{BUMP}} version: $current_version → $new_version"
    
    # Create and switch to release branch first
    echo "📦 Creating release branch..."
    git checkout -b release 2>/dev/null || git checkout release
    git pull origin main --no-edit
    
    # Update Cargo.toml with new version on release branch
    sed -i.bak "s/^version = \"$current_version\"/version = \"$new_version\"/" Cargo.toml
    rm Cargo.toml.bak
    
    # Update Cargo.lock to reflect the new version
    cargo check --quiet
    
    # Commit version bump to release branch
    git add Cargo.toml Cargo.lock
    git commit -m "Bump version to $new_version for release"
    
    # Push release branch to trigger GitHub Actions
    echo "🚀 Pushing to release branch to trigger GitHub Actions..."
    git push origin release
    
    # Switch back to main
    git checkout main
    
    echo "✅ Release $new_version triggered! Monitor at: https://github.com/2mawi2/para/actions"
    echo "💡 The release workflow will automatically merge back to main when complete"

# Refresh Claude credentials for GitHub Actions (idempotent)
refresh-claude-secrets:
    #!/usr/bin/env bash
    set -euo pipefail
    
    echo "Refreshing Claude credentials for GitHub Actions..."
    
    # Get credentials from macOS keychain
    CREDS=$(security find-generic-password -s "Claude Code-credentials" -a "$USER" -w 2>/dev/null || echo "")
    
    if [ -z "$CREDS" ]; then
        echo "Error: Claude credentials not found in keychain"
        echo "   Please ensure you're logged in to Claude Code with /login"
        exit 1
    fi
    
    # Parse JSON to extract tokens
    ACCESS_TOKEN=$(echo "$CREDS" | python3 -c "import sys, json; print(json.load(sys.stdin)['claudeAiOauth']['accessToken'])")
    REFRESH_TOKEN=$(echo "$CREDS" | python3 -c "import sys, json; print(json.load(sys.stdin)['claudeAiOauth']['refreshToken'])")
    EXPIRES_AT=$(echo "$CREDS" | python3 -c "import sys, json; print(json.load(sys.stdin)['claudeAiOauth']['expiresAt'])")
    
    # Get repository from git remote
    REPO=$(git remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git$/\1/' | sed 's/.*github.com[:/]\(.*\)$/\1/')
    
    echo "Updating secrets for repository: $REPO"
    
    # Set GitHub secrets (idempotent - will update if exists)
    gh secret set CLAUDE_ACCESS_TOKEN --repo "$REPO" --body "$ACCESS_TOKEN"
    gh secret set CLAUDE_REFRESH_TOKEN --repo "$REPO" --body "$REFRESH_TOKEN"
    gh secret set CLAUDE_EXPIRES_AT --repo "$REPO" --body "$EXPIRES_AT"
    
    echo "Claude credentials successfully refreshed!"
    echo ""
    echo "GitHub Actions configured:"
    echo "   - Claude PR Assistant (responds to @claude mentions)"
    echo "   - Claude Auto Review (reviews all PRs automatically)"