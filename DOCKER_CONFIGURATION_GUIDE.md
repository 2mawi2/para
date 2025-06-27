# Docker Configuration Guide for Para

## Overview

Para now supports two powerful Docker customization features:
1. **Custom Docker Images** - Use any Docker image instead of the default
2. **Setup Scripts** - Run initialization scripts when containers start

## Configuration Methods

### Method 1: Command Line (Highest Priority)

```bash
# Custom Docker image with setup script
para start my-session --container \
  --docker-image python:3.11 \
  --setup-script scripts/setup.sh

# Custom image without API keys (for untrusted images)
para start my-session --container \
  --docker-image untrusted:latest \
  --no-forward-keys

# Just setup script with default image
para start my-session --container \
  --setup-script .para/dev-setup.sh
```

### Method 2: Auto-Detection

Para automatically looks for `.para/setup.sh` in your repository root:

```bash
# Create setup script
mkdir -p .para
cat > .para/setup.sh << 'EOF'
#!/bin/bash
echo "🚀 Setting up development environment..."

# Load .env file if it exists
if [ -f "$PARA_WORKSPACE/.env" ]; then
    set -a
    source "$PARA_WORKSPACE/.env"
    set +a
fi

# Install dependencies based on project type
if [ -f "$PARA_WORKSPACE/package.json" ]; then
    npm install
elif [ -f "$PARA_WORKSPACE/requirements.txt" ]; then
    pip install -r "$PARA_WORKSPACE/requirements.txt"
fi

echo "✅ Setup complete!"
EOF

chmod +x .para/setup.sh

# Now any container session will auto-run this script
para start my-session --container
```

### Method 3: Global Configuration

Edit your para config file:

```bash
# Show current config
para config show

# Edit config manually
# macOS: ~/Library/Application Support/para/config.json
# Linux: ~/.config/para/config.json
# Windows: %APPDATA%\para\config.json
```

Add Docker configuration:

```json
{
  "ide": { ... },
  "docker": {
    "setup_script": ".para/setup.sh",
    "default_image": "mycompany/dev-env:latest",
    "forward_env_keys": [
      "ANTHROPIC_API_KEY",
      "OPENAI_API_KEY",
      "GITHUB_TOKEN",
      "CUSTOM_API_KEY"
    ]
  }
}
```

## Priority Order

### For Docker Images:
1. CLI flag `--docker-image` (highest priority)
2. Config `docker.default_image`
3. Default: `para-authenticated:latest`

### For Setup Scripts:
1. CLI flag `--setup-script` (highest priority)
2. Auto-detected `.para/setup.sh` (if exists)
3. Config `docker.setup_script`
4. No script (if none configured)

## Recommended Docker Images

### Option 1: Pre-built Para Development Image (Recommended)

Build a custom image with all dependencies pre-installed:

```bash
# One-time setup: Build the para-dev image
cd /path/to/para
./docker/build-para-dev-image.sh

# Now use it for development (starts instantly!)
para start my-feature --container --docker-image para-dev:latest
```

The `para-dev:latest` image includes:
- Ubuntu 22.04 base
- Rust toolchain (latest stable)
- Just command runner
- Node.js 20.x and npm
- Bun (optional, faster npm alternative)
- All system dependencies for building para

### Option 2: Use Base Images (Slower Initial Setup)

If you prefer not to build a custom image:

```bash
# Ubuntu (requires full setup - 5-10 minutes first run)
para start test-docker-dev --container --docker-image ubuntu:22.04

# Rust official image (faster - Rust pre-installed)
para start test-docker-dev --container --docker-image rust:latest
```

### Development Workflow with Pre-built Image

```bash
# 1. Build the image once
./docker/build-para-dev-image.sh

# 2. Start development sessions instantly
para start feature-x --container --docker-image para-dev:latest

# 3. Inside the container, everything is ready
just test     # Run all tests
just build    # Build para
just fmt      # Format code
just lint     # Run linting

# 4. When done, create your branch
para finish "Add feature X"
```

## Example Setup Scripts

### Basic Development Setup
```bash
#!/bin/bash
echo "🚀 Para Setup: $PARA_SESSION"

# Install dependencies
if [ -f package.json ]; then
    npm install
fi

# Set up git config
git config user.name "Your Name"
git config user.email "your.email@example.com"

# Create useful directories
mkdir -p logs temp

echo "✅ Ready for development!"
```

### Python ML Environment
```bash
#!/bin/bash
# Install Python dependencies
pip install -r requirements.txt

# Download models if needed
if [ ! -d "models" ]; then
    echo "📥 Downloading ML models..."
    wget https://example.com/models.tar.gz
    tar -xzf models.tar.gz
    rm models.tar.gz
fi

# Set up Jupyter
jupyter notebook --generate-config
```

### Multi-Language Setup
```bash
#!/bin/bash
# Detect and install based on project files
if [ -f "Cargo.toml" ]; then
    cargo build
fi

if [ -f "go.mod" ]; then
    go mod download
fi

if [ -f "composer.json" ]; then
    composer install
fi

# Load environment variables
[ -f .env ] && export $(cat .env | xargs)
```

## Security Considerations

### API Key Forwarding
- By default, Para forwards common API keys to containers
- Use `--no-forward-keys` when using untrusted images
- Configure specific keys via `docker.forward_env_keys`

### Custom Images
- Only use trusted Docker images when API keys are forwarded
- Para shows warnings when using custom images with API keys
- Consider creating your own base images for security

## Common Use Cases

### 1. Node.js Development
```bash
# Use official Node image with auto-setup
echo '#!/bin/bash
npm install
npm run prepare
' > .para/setup.sh
chmod +x .para/setup.sh

para start feature-x --container --docker-image node:18
```

### 2. Python Data Science
```bash
# Configure globally for all sessions
para config set docker.default_image "python:3.11-slim"
para config set docker.setup_script "scripts/ml-setup.sh"

# Now all container sessions use Python image
para start analysis --container
```

### 3. Secure External Testing
```bash
# Use untrusted image without API keys
para start test-external --container \
  --docker-image someuser/unknown:latest \
  --no-forward-keys
```

## Important Notes

### Setup Scripts in Regular Sessions
**No**, setup scripts only run in Docker container sessions (`--container` flag). They do NOT run for regular git worktree sessions because:
- Regular sessions run on your host machine
- No isolation boundary to set up
- Your host environment is already configured

### Alpine Linux Compatibility
If using Alpine-based images, modify your setup scripts to use `sh` instead of `bash`:
```bash
#!/bin/sh
# Alpine-compatible setup script
echo "Setting up in Alpine..."
```

### Error Handling
Setup script failures will prevent the session from starting. Make scripts robust:
```bash
#!/bin/bash
set -e  # Exit on error

# Check prerequisites
command -v npm >/dev/null 2>&1 || { echo "npm required"; exit 1; }

# Continue with setup...
```

## Troubleshooting

### Script Not Running
1. Check script exists and is executable: `ls -la .para/setup.sh`
2. Verify script path in error messages
3. Check container logs: `docker logs para-<session-name>`

### Permission Errors
```bash
# Ensure script is executable
chmod +x .para/setup.sh

# For config directory issues
mkdir -p "$(para config show | jq -r .directories.state_dir)"
```

### Image Pull Failures
- Ensure Docker daemon is running
- Check image name spelling
- Verify network connectivity
- Login to private registries: `docker login`

## Best Practices

1. **Version Control**: Commit `.para/setup.sh` to your repository
2. **Idempotency**: Make scripts safe to run multiple times
3. **Fast Execution**: Keep setup scripts quick (<30 seconds)
4. **Error Messages**: Provide clear output for debugging
5. **Documentation**: Comment complex setup steps

## Migration from Existing Workflow

If you have existing Docker workflows, integrate them:

```bash
# Old: manual Docker setup
docker run -it -v $(pwd):/workspace myimage bash
# Then manually: npm install, etc.

# New: automated with para
echo '#!/bin/bash
# Your existing setup commands here
npm install
npm run prepare
' > .para/setup.sh

para start --container --docker-image myimage
```

The setup script runs automatically after container creation!