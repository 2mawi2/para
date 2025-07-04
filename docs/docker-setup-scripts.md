# Docker Setup Scripts

Para supports running custom setup scripts inside Docker containers after they start. This enables environment configuration, dependency installation, and local file creation without modifying the base Docker image.

## Overview

Setup scripts allow you to:
- Install project-specific dependencies
- Configure environment variables
- Create local configuration files
- Set up development databases
- Initialize test data
- Configure IDE extensions or settings

## How to Use Setup Scripts

### 1. Command Line Flag (Highest Priority)

Specify a setup script directly when starting a session:

```bash
# With start command
para start --container --setup-script scripts/dev-setup.sh

# With AI-assisted start command
para start --container --setup-script scripts/init-env.sh "implement feature X"
```

### 2. Default Script Location

Para automatically looks for `.para/setup.sh` in your repository root:

```bash
# Create the default setup script
mkdir -p .para
cat > .para/setup.sh << 'EOF'
#!/bin/bash
echo "🚀 Setting up development environment..."

# Your setup commands here
npm install
echo "✅ Setup complete!"
EOF

chmod +x .para/setup.sh

# Now any container session will run this script
para start --container
```

### 3. Configuration File

Set a default setup script path in your para configuration:

```json
{
  "ide": { ... },
  "docker": {
    "setup_script": "scripts/container-init.sh"
  }
}
```

## Priority Order

When multiple setup scripts are available, para uses this priority order:

1. CLI `--setup-script` flag (highest priority)
2. `.para/setup.sh` (if exists)
3. Config `docker.setup_script`
4. No script (skip setup)

## Environment Variables

Setup scripts have access to these environment variables:

- `PARA_WORKSPACE`: The workspace directory inside the container (always `/workspace`)
- `PARA_SESSION`: The current para session name

## Example Setup Scripts

### Basic Node.js Setup

```bash
#!/bin/bash
# .para/setup.sh

echo "🚀 Setting up Node.js environment..."

# Load environment variables from .env file
if [ -f "$PARA_WORKSPACE/.env" ]; then
    echo "📋 Loading environment variables..."
    set -a
    source "$PARA_WORKSPACE/.env"
    set +a
fi

# Install dependencies
if [ -f "$PARA_WORKSPACE/package.json" ]; then
    echo "📦 Installing npm dependencies..."
    cd "$PARA_WORKSPACE" && npm install
fi

# Create local config if it doesn't exist
if [ ! -f "$PARA_WORKSPACE/.env.local" ]; then
    echo "🔧 Creating local configuration..."
    cat > "$PARA_WORKSPACE/.env.local" << EOF
DATABASE_URL=postgresql://localhost/dev_${PARA_SESSION}
API_KEY=dev-key-${PARA_SESSION}
NODE_ENV=development
EOF
fi

echo "✅ Node.js setup complete!"
```

### Python Development Setup

```bash
#!/bin/bash
# scripts/python-setup.sh

echo "🐍 Setting up Python environment..."

cd "$PARA_WORKSPACE"

# Create virtual environment
if [ ! -d "venv" ]; then
    echo "📦 Creating virtual environment..."
    python -m venv venv
fi

# Activate and install dependencies
source venv/bin/activate

if [ -f "requirements.txt" ]; then
    echo "📋 Installing Python dependencies..."
    pip install -r requirements.txt
fi

if [ -f "requirements-dev.txt" ]; then
    echo "🔧 Installing development dependencies..."
    pip install -r requirements-dev.txt
fi

# Set up pre-commit hooks
if [ -f ".pre-commit-config.yaml" ]; then
    echo "🪝 Setting up pre-commit hooks..."
    pre-commit install
fi

echo "✅ Python setup complete!"
```

### Database Initialization

```bash
#!/bin/bash
# .para/setup.sh

echo "🗄️ Setting up development database..."

# Wait for database to be ready (if using docker-compose)
if command -v docker-compose &> /dev/null; then
    echo "⏳ Waiting for database..."
    docker-compose up -d db
    sleep 5
fi

# Run migrations
if [ -f "$PARA_WORKSPACE/manage.py" ]; then
    echo "🔄 Running database migrations..."
    cd "$PARA_WORKSPACE"
    python manage.py migrate
    
    # Load fixtures
    if [ -d "fixtures" ]; then
        echo "📊 Loading test data..."
        python manage.py loaddata fixtures/*.json
    fi
fi

echo "✅ Database setup complete!"
```

### Multi-Language Project Setup

```bash
#!/bin/bash
# scripts/full-stack-setup.sh

echo "🚀 Setting up full-stack environment for session: $PARA_SESSION"

cd "$PARA_WORKSPACE"

# Backend setup (Node.js)
if [ -f "backend/package.json" ]; then
    echo "📦 Setting up backend..."
    cd backend
    npm install
    
    # Copy and configure environment
    if [ ! -f ".env" ] && [ -f ".env.example" ]; then
        cp .env.example .env
        # Update database name to include session
        sed -i "s/myapp_dev/myapp_${PARA_SESSION}/g" .env
    fi
    
    cd ..
fi

# Frontend setup (React)
if [ -f "frontend/package.json" ]; then
    echo "⚛️ Setting up frontend..."
    cd frontend
    npm install
    
    # Create local env file
    if [ ! -f ".env.local" ]; then
        echo "REACT_APP_API_URL=http://localhost:3001" > .env.local
        echo "REACT_APP_SESSION=$PARA_SESSION" >> .env.local
    fi
    
    cd ..
fi

# Database setup
if command -v docker &> /dev/null; then
    echo "🐳 Starting local database..."
    docker run -d \
        --name "postgres-$PARA_SESSION" \
        -e POSTGRES_DB="myapp_${PARA_SESSION}" \
        -e POSTGRES_PASSWORD=devpassword \
        -p 5432:5432 \
        postgres:15
fi

# Install global tools
echo "🔧 Installing development tools..."
npm install -g nodemon prettier eslint

echo "✅ Full-stack setup complete!"
echo ""
echo "📝 Next steps:"
echo "   - Backend: cd backend && npm run dev"
echo "   - Frontend: cd frontend && npm start"
echo "   - Database: postgresql://localhost/myapp_${PARA_SESSION}"
```

### IDE Configuration Setup

```bash
#!/bin/bash
# .para/setup.sh

echo "💻 Configuring IDE settings..."

# Create VS Code settings for the workspace
mkdir -p "$PARA_WORKSPACE/.vscode"

if [ ! -f "$PARA_WORKSPACE/.vscode/settings.json" ]; then
    cat > "$PARA_WORKSPACE/.vscode/settings.json" << 'EOF'
{
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "esbenp.prettier-vscode",
    "eslint.validate": ["javascript", "typescript"],
    "python.linting.enabled": true,
    "python.linting.pylintEnabled": true,
    "python.formatting.provider": "black"
}
EOF
fi

# Install recommended extensions list
if [ ! -f "$PARA_WORKSPACE/.vscode/extensions.json" ]; then
    cat > "$PARA_WORKSPACE/.vscode/extensions.json" << 'EOF'
{
    "recommendations": [
        "dbaeumer.vscode-eslint",
        "esbenp.prettier-vscode",
        "ms-python.python",
        "ms-python.vscode-pylance"
    ]
}
EOF
fi

echo "✅ IDE configuration complete!"
```

## Best Practices

1. **Make Scripts Idempotent**: Scripts should be safe to run multiple times without causing issues.

2. **Use Session Names**: Leverage `$PARA_SESSION` to create session-specific resources (databases, config files, etc.).

3. **Handle Errors Gracefully**: Use conditional checks and provide clear error messages.

4. **Keep Scripts Fast**: Long-running setup scripts delay IDE startup. Consider running heavy tasks in the background.

5. **Version Control**: Commit your `.para/setup.sh` to version control so all team members have the same setup.

6. **Environment Detection**: Check for command availability before using them:
   ```bash
   if command -v npm &> /dev/null; then
       npm install
   fi
   ```

7. **Progress Feedback**: Use echo statements to show progress, especially for longer operations.

## Troubleshooting

### Script Not Running

1. Check the script exists and has correct permissions:
   ```bash
   ls -la .para/setup.sh
   chmod +x .para/setup.sh
   ```

2. Verify the path is correct when using `--setup-script`

3. Check para's output for error messages

### Script Failures

If a setup script fails:
- The container continues running
- Error output is displayed in the terminal
- You can manually run commands inside the container:
  ```bash
  docker exec -it para-<session-name> bash
  ```

### Debugging Scripts

Add debugging to your scripts:
```bash
#!/bin/bash
set -x  # Print commands as they execute
set -e  # Exit on first error

echo "Current directory: $(pwd)"
echo "Environment: PARA_SESSION=$PARA_SESSION"
```

## Security Considerations

- Setup scripts run with the same permissions as the container user
- Avoid hardcoding secrets in setup scripts
- Use environment variables or mounted secrets for sensitive data
- Be cautious with scripts that download and execute code

## Integration with CI/CD

Setup scripts can be used in CI/CD pipelines to ensure consistent environments:

```yaml
# .github/workflows/test.yml
- name: Run tests in para container
  run: |
    para start test-session --container --setup-script ci/test-setup.sh
    docker exec para-test-session npm test
```