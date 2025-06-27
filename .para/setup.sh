#!/bin/bash
# Para Development Environment Setup Script
# This script sets up everything needed to build and test the para project inside Docker containers

set -euo pipefail

echo "🚀 Setting up para development environment..."
echo "   Session: ${PARA_SESSION:-unknown}"
echo "   Workspace: ${PARA_WORKSPACE:-/workspace}"
echo ""

# Change to workspace directory
cd "${PARA_WORKSPACE:-/workspace}"

# Update package manager and install system dependencies
echo "📦 Installing system dependencies..."
apt-get update -qq
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    git \
    ca-certificates \
    wget

# Install Rust if not already installed
if ! command -v rustc &> /dev/null; then
    echo "🦀 Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
else
    echo "✅ Rust already installed: $(rustc --version)"
fi

# Ensure cargo is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Install just command runner
if ! command -v just &> /dev/null; then
    echo "📦 Installing just command runner..."
    # Use prebuilt binary for speed
    JUST_VERSION="1.16.0"
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64) ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *) echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
    esac
    
    wget -q "https://github.com/casey/just/releases/download/${JUST_VERSION}/just-${JUST_VERSION}-${ARCH}-unknown-linux-musl.tar.gz"
    tar xzf "just-${JUST_VERSION}-${ARCH}-unknown-linux-musl.tar.gz"
    mv just /usr/local/bin/
    rm "just-${JUST_VERSION}-${ARCH}-unknown-linux-musl.tar.gz"
    echo "✅ just installed: $(just --version)"
else
    echo "✅ just already installed: $(just --version)"
fi

# Install Node.js and npm for TypeScript MCP server
if ! command -v node &> /dev/null; then
    echo "📦 Installing Node.js..."
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
    apt-get install -y -qq nodejs
    echo "✅ Node.js installed: $(node --version)"
else
    echo "✅ Node.js already installed: $(node --version)"
fi

# Install bun (optional, faster than npm)
if ! command -v bun &> /dev/null; then
    echo "📦 Installing bun (optional)..."
    if curl -fsSL https://bun.sh/install | bash; then
        export PATH="$HOME/.bun/bin:$PATH"
        echo "✅ bun installed: $(bun --version)"
    else
        echo "⚠️  bun installation failed, will use npm"
    fi
else
    echo "✅ bun already installed: $(bun --version)"
fi

# Configure git (required for some operations)
echo "🔧 Configuring git..."
git config --global user.email "para-dev@example.com" 2>/dev/null || true
git config --global user.name "Para Developer" 2>/dev/null || true
git config --global init.defaultBranch main 2>/dev/null || true

# Build para
echo ""
echo "🦀 Building para..."
if [ -f "Cargo.toml" ]; then
    # Download dependencies first (helps with caching)
    cargo fetch
    
    # Run initial build
    if just build; then
        echo "✅ Para built successfully"
    else
        echo "⚠️  Build failed, but continuing setup..."
    fi
else
    echo "❌ No Cargo.toml found in workspace"
fi

# Install TypeScript dependencies if MCP server exists
if [ -d "mcp-server-ts" ] && [ -f "mcp-server-ts/package.json" ]; then
    echo "📦 Installing TypeScript MCP server dependencies..."
    cd mcp-server-ts
    
    if command -v bun &> /dev/null; then
        bun install
        echo "✅ TypeScript dependencies installed with bun"
    elif command -v npm &> /dev/null; then
        npm install
        echo "✅ TypeScript dependencies installed with npm"
    else
        echo "❌ Neither bun nor npm found for TypeScript dependencies"
    fi
    
    cd ..
fi

# Run tests to verify setup
echo ""
echo "🧪 Running tests to verify setup..."
if just test; then
    echo "✅ All tests passed!"
else
    echo "⚠️  Some tests failed, but environment is set up"
fi

# Display helpful information
echo ""
echo "✅ Para development environment ready!"
echo ""
echo "📝 Available commands:"
echo "   just test             - Run all tests with linting and formatting"
echo "   just test <filter>    - Run specific tests (e.g., 'just test docker')"
echo "   just build            - Build debug binary"
echo "   just build-release    - Build optimized release binary"
echo "   just lint             - Run Rust linting (clippy)"
echo "   just fmt              - Format Rust code"
echo "   just install          - Install para globally"
echo ""
echo "🐳 Docker-specific testing:"
echo "   cargo test docker     - Run Docker-related tests"
echo "   just test integration - Run integration tests"
echo ""
echo "💡 Tips:"
echo "   - This container has all dependencies needed for para development"
echo "   - Your code is mounted at ${PARA_WORKSPACE:-/workspace}"
echo "   - Changes you make are reflected on your host machine"
echo "   - Use 'para finish' when done to create a branch for review"