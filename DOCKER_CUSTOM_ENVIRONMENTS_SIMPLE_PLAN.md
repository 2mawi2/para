# Para Docker Custom Environments - Two-Option Plan

## Overview

This plan provides two distinct options for customizing Docker environments in para:
1. **Custom Docker Image** - Override the container image for system dependencies
2. **Setup Scripts** - Initialize the development environment after container starts

## Option 1: Custom Docker Image Override

### Implementation (~20 lines of code)

```rust
// In src/cli/parser.rs - add flag
#[arg(long = "docker-image")]
pub docker_image: Option<String>,

// In src/config/mod.rs - add to Config struct
#[derive(Deserialize, Serialize)]
pub struct DockerConfig {
    pub default_image: Option<String>,
}

// In DockerManager::get_docker_image() - check for override
pub fn get_docker_image(&self, cli_image: Option<String>) -> String {
    // Priority: CLI > Config > Default
    cli_image
        .or_else(|| self.config.docker.as_ref()?.default_image.clone())
        .unwrap_or_else(|| "para-authenticated:latest".to_string())
}
```

### User Usage

```bash
# Via CLI
para start --container --docker-image mycompany/dev-env:latest

# Via config (.para/config.json)
{
  "docker": {
    "default_image": "mycompany/dev-env:latest"
  }
}
```

## Option 2: Setup Scripts

### Implementation (~30 lines of code)

```rust
// In src/cli/parser.rs
#[arg(long = "setup-script")]
pub setup_script: Option<PathBuf>,

// In src/config/mod.rs
pub struct DockerConfig {
    pub default_image: Option<String>,
    pub setup_script: Option<String>,  // Path to default setup script
}

// Setup script execution in DockerService
impl DockerService {
    pub async fn run_setup_script(&self, container_id: &str, script_path: &Path) -> Result<()> {
        let script_content = fs::read_to_string(script_path)?;
        
        // Create exec with proper environment
        let exec_config = CreateExecOptions {
            cmd: Some(vec!["bash", "-c", &script_content]),
            working_dir: Some("/workspace"),
            env: Some(vec![
                "PARA_WORKSPACE=/workspace",
                "PARA_SESSION={session_name}",
                "PARA_HOST_ENV_PATH=/host-env",  // Mounted host env directory
            ]),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };
        
        // Execute and stream output
        let exec = self.docker.create_exec(container_id, exec_config).await?;
        // ... stream output ...
        Ok(())
    }
}

// In container startup - determine which script to run
let script_path = args.setup_script
    .or_else(|| Path::new(".para/setup.sh").exists().then(|| PathBuf::from(".para/setup.sh")))
    .or_else(|| config.docker.as_ref()?.setup_script.as_ref().map(PathBuf::from));

if let Some(script) = script_path {
    docker_service.run_setup_script(&container_id, &script).await?;
}
```

### Setup Script Priority Order
1. CLI `--setup-script` flag (highest)
2. `.para/setup.sh` if exists (default)
3. Config file `docker.setup_script`
4. No script (skip setup)

## Setup Script Guide & Examples

### Basic Template (.para/setup.sh)

```bash
#!/bin/bash
# Para Development Environment Setup Script
# This runs inside the container after it starts

echo "🚀 Setting up para development environment..."

# === ENVIRONMENT VARIABLES ===
# Option 1: Load from .env file in workspace
if [ -f "$PARA_WORKSPACE/.env" ]; then
    echo "📋 Loading environment from .env file..."
    set -a  # Export all variables
    source "$PARA_WORKSPACE/.env"
    set +a
fi

# Option 2: Copy specific env vars from host
# (Requires passing host env via docker -e flags)
export API_KEY="${HOST_API_KEY:-$API_KEY}"

# === PROJECT DEPENDENCIES ===
cd "$PARA_WORKSPACE"

# Node.js project
if [ -f "package.json" ]; then
    echo "📦 Installing npm dependencies..."
    npm install
fi

# Python project
if [ -f "requirements.txt" ]; then
    echo "🐍 Installing Python dependencies..."
    pip install -r requirements.txt
fi

# === SETUP LOCAL FILES ===
# Create files that aren't in git
if [ ! -f ".env.local" ]; then
    echo "📝 Creating .env.local file..."
    cat > .env.local << EOF
# Local development settings
DATABASE_URL=postgresql://localhost/dev_db
REDIS_URL=redis://localhost:6379
SECRET_KEY=$(openssl rand -hex 32)
EOF
fi

# === GIT CONFIGURATION ===
git config --global user.name "Para Agent"
git config --global user.email "para@localhost"

echo "✅ Environment setup complete!"
```

### Advanced Examples

#### 1. Multi-Environment Setup
```bash
#!/bin/bash
# .para/setup-dev.sh - Development environment

case "${PARA_ENV:-development}" in
    production)
        cp .env.production .env
        npm run build
        ;;
    staging)
        cp .env.staging .env
        npm run build:staging
        ;;
    *)
        cp .env.development .env
        npm install
        npm run prepare
        ;;
esac
```

#### 2. Secret Management
```bash
#!/bin/bash
# Setup secrets from host environment

# Option 1: Use environment variables passed from host
if [ -n "$ANTHROPIC_API_KEY" ]; then
    echo "ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY" >> .env.local
fi

# Option 2: Fetch from secret manager
if command -v op &> /dev/null; then
    echo "🔐 Fetching secrets from 1Password..."
    op item get "Para Dev Secrets" --fields label=api_key > .env.secrets
fi

# Option 3: Interactive setup
if [ -t 0 ] && [ -z "$CI" ]; then
    read -p "Enter API key (or press Enter to skip): " api_key
    if [ -n "$api_key" ]; then
        echo "API_KEY=$api_key" >> .env.local
    fi
fi
```

#### 3. Database Setup
```bash
#!/bin/bash
# Initialize development database

# Start PostgreSQL if needed
if ! pg_isready -h localhost; then
    echo "🐘 Starting PostgreSQL..."
    docker run -d --name para-postgres -p 5432:5432 \
        -e POSTGRES_PASSWORD=dev postgres:14
fi

# Run migrations
echo "🔄 Running database migrations..."
npm run db:migrate

# Seed development data
if [ "$SEED_DB" = "true" ]; then
    npm run db:seed
fi
```

## Configuration Examples

### Repository Config (.para/config.json)
```json
{
  "docker": {
    "default_image": "node:18-alpine",
    "setup_script": ".para/setup-dev.sh"
  }
}
```

### Global Config (~/.para/config.json)
```json
{
  "docker": {
    "default_image": "mycompany/standard-dev:latest"
  }
}
```

## Usage Patterns

### 1. Simple Project
```bash
# Just use default image and setup
para start --container
# Automatically runs .para/setup.sh if exists
```

### 2. Custom Image + Setup
```bash
# Use company standard image with project setup
para start --container --docker-image company/python:3.11
# Still runs .para/setup.sh for project-specific setup
```

### 3. Different Environments
```bash
# Development
para start --container --setup-script .para/setup-dev.sh

# Testing
para start --container --setup-script .para/setup-test.sh

# Production-like
para start --container --docker-image prod-image:latest --setup-script .para/setup-prod.sh
```

## API Key Forwarding

Still implement minimal automatic forwarding:

```rust
// Auto-forward essential API keys
const API_KEYS: &[&str] = &["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GITHUB_TOKEN"];
for key in API_KEYS {
    if let Ok(value) = std::env::var(key) {
        env_vars.push(format!("{}={}", key, value));
    }
}
```

## Implementation Summary

Total code changes: ~50 lines
- Custom image override: ~20 lines
- Setup script support: ~30 lines
- Minimal API key forwarding: included

This gives users:
1. **System dependencies** via custom Docker images
2. **Environment setup** via flexible bash scripts
3. **Configuration options** via CLI and config files
4. **Automatic defaults** that just work

## Best Practices for Users

1. **Use custom images for system dependencies** (apt packages, languages, tools)
2. **Use setup scripts for project setup** (env vars, local files, services)
3. **Keep setup scripts idempotent** (can run multiple times safely)
4. **Version control setup scripts** but not generated files
5. **Use .para/ directory** for para-specific files