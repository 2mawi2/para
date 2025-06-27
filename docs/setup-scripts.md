# Setup Scripts

Para supports running custom setup scripts when creating new worktree sessions. This allows you to automate initialization tasks like installing dependencies, setting up environments, or configuring tools.

## Quick Start

Para supports environment-specific setup scripts for different development workflows:

### For Docker Development
Create `.para/setup-docker.sh` for container-based sessions:

```bash
#!/bin/bash
echo "Setting up Docker session: $PARA_SESSION"
echo "Working in: $PARA_WORKSPACE"

# Install dependencies in container
apt-get update && apt-get install -y build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

### For Regular Worktrees  
Create `.para/setup-worktree.sh` for native development:

```bash
#!/bin/bash
echo "Setting up worktree session: $PARA_SESSION"
echo "Working in: $PARA_WORKSPACE"

# Your setup commands here
npm install
echo "Ready for development!"
```

### Generic Setup
Create `.para/setup.sh` for both environments (fallback):

```bash
#!/bin/bash
echo "Setting up session: $PARA_SESSION"
npm install
```

Scripts run automatically when you start sessions:

```bash
para start my-feature              # Uses .para/setup-worktree.sh
para start my-feature --container  # Uses .para/setup-docker.sh
```

## Configuration Options

### Priority Order

Setup scripts are discovered in the following order (first match wins):

1. **CLI Flag**: `para start --setup-script scripts/custom-setup.sh`
2. **Environment-Specific Default**:
   - Docker: `.para/setup-docker.sh`
   - Worktree: `.para/setup-worktree.sh`
3. **Generic Default**: `.para/setup.sh` (fallback for both environments)
4. **Config File**: Set `setup_script` or `docker.setup_script` in your para config

### Config File Setup

Add to your para config (`~/.config/para/config.json` on macOS/Linux):

```json
{
  "setup_script": ".para/project-setup.sh",
  // ... other config
}
```

For Docker containers, you can specify a different script:

```json
{
  "docker": {
    "setup_script": ".para/docker-setup.sh"
  }
}
```

## Environment Variables

Setup scripts receive these environment variables:

- `PARA_WORKSPACE`: Full path to the worktree directory
- `PARA_SESSION`: Name of the current para session

## Examples

### Copy .env File from Main Repository

A common use case is copying environment configuration from the main repository to each worktree:

```bash
#!/bin/bash
# .para/setup.sh

echo "🔧 Setting up environment for session: $PARA_SESSION"

# Para worktrees are in <repo>/.para/worktrees/<session-name>
# Go up 3 levels to reach the main repository root
MAIN_REPO_ENV="$(dirname $(dirname $(dirname "$PARA_WORKSPACE")))/.env"

# Copy .env file if it exists
if [ -f "$MAIN_REPO_ENV" ] && [ ! -f "$PARA_WORKSPACE/.env" ]; then
    echo "📋 Copying .env file from main repository..."
    cp "$MAIN_REPO_ENV" "$PARA_WORKSPACE/.env"
    echo "✅ .env file copied successfully"
elif [ -f "$PARA_WORKSPACE/.env" ]; then
    echo "ℹ️  .env file already exists in worktree"
else
    echo "⚠️  No .env file found in main repository"
fi
```

### Basic Dependencies Setup

```bash
#!/bin/bash
# .para/setup.sh

# Install Node.js dependencies
if [ -f "package.json" ]; then
    echo "📦 Installing npm dependencies..."
    npm install
fi

# Set up Python environment
if [ -f "requirements.txt" ]; then
    echo "🐍 Setting up Python virtual environment..."
    python3 -m venv .venv
    source .venv/bin/activate
    pip install -r requirements.txt
fi

# Run database migrations
if [ -f "manage.py" ]; then
    echo "🗄️ Running migrations..."
    python manage.py migrate
fi
```

### Environment-Specific Setup

```bash
#!/bin/bash
# .para/setup.sh

# Copy environment template
if [ ! -f ".env" ] && [ -f ".env.example" ]; then
    echo "📋 Creating .env file from template..."
    cp .env.example .env
    echo "⚠️  Please update .env with your local settings"
fi

# Generate development certificates
if [ ! -d "certs" ]; then
    echo "🔒 Generating development certificates..."
    mkdir -p certs
    mkcert -install
    mkcert -cert-file certs/localhost.pem -key-file certs/localhost-key.pem localhost
fi
```

### Tool Installation

```bash
#!/bin/bash
# .para/setup.sh

# Ensure required tools are available
command -v node >/dev/null 2>&1 || {
    echo "❌ Node.js is required but not installed."
    exit 1
}

# Install project-specific tools
if ! command -v pnpm >/dev/null 2>&1; then
    echo "📦 Installing pnpm..."
    npm install -g pnpm
fi

# Set up git hooks
if [ -f ".husky/install.sh" ]; then
    echo "🪝 Setting up git hooks..."
    .husky/install.sh
fi
```

## Security Considerations

⚠️ **Important**: Setup scripts run with your full user permissions!

- Only run scripts from trusted sources
- Review script contents before execution
- Be cautious with scripts that:
  - Download files from the internet
  - Modify system configuration
  - Access sensitive data

Para displays the script path before execution as a security reminder.

## Usage with Dispatch

Setup scripts also work with `para dispatch`:

```bash
# With explicit script
para dispatch --setup-script scripts/ai-setup.sh "implement new feature"

# Auto-detect .para/setup.sh
para dispatch "fix bug in authentication"
```

## Docker Container Support

The same setup script functionality works for Docker containers:

```bash
# Scripts run inside the container after creation
para start --container --setup-script docker-init.sh
```

For Docker, consider container-specific setup:

```bash
#!/bin/bash
# .para/docker-setup.sh

# Install system packages (if root)
if [ "$EUID" -eq 0 ]; then
    apt-get update
    apt-get install -y postgresql-client redis-tools
fi

# Container-specific environment setup
export DATABASE_HOST=host.docker.internal
export REDIS_HOST=host.docker.internal
```

## Troubleshooting

### Script Not Found

If your script isn't detected:

1. Check file exists: `ls -la .para/setup.sh`
2. Verify path in error message
3. Use absolute path with `--setup-script` flag

### Script Fails to Execute

If the script fails:

1. Check execute permissions: `chmod +x .para/setup.sh`
2. Verify bash is available: `which bash`
3. Run script manually to debug: `bash .para/setup.sh`

### Exit Codes

- Script must exit with code 0 for success
- Any non-zero exit code will stop session creation
- Para displays the exit code in error messages

## Best Practices

1. **Make scripts idempotent**: Should be safe to run multiple times
2. **Check prerequisites**: Verify required tools before using them
3. **Provide feedback**: Use echo statements to show progress
4. **Handle errors**: Use `set -e` to exit on errors
5. **Keep it fast**: Long-running scripts delay IDE launch

Example template:

```bash
#!/bin/bash
set -e  # Exit on error

echo "🔧 Setting up $PARA_SESSION..."

# Your setup commands here

echo "✅ Setup complete!"
```