# Para Docker Custom Environments - Simple Implementation Plan

## Overview

This plan provides the simplest possible approach for users to create custom Docker environments with para, requiring minimal code changes while providing maximum flexibility.

## The Single-File Solution: `.para/docker-env.sh`

Instead of implementing multiple features (env files, Dockerfiles, volume mounts), we use a single setup script that handles everything.

### Core Implementation (< 50 lines of code)

#### 1. Minimal API Key Forwarding
```rust
// In DockerService::create_container() - add these 5 lines
const ESSENTIAL_KEYS: &[&str] = &["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GITHUB_TOKEN"];
for key in ESSENTIAL_KEYS {
    if let Ok(value) = std::env::var(key) {
        env_vars.push(format!("{}={}", key, value));
    }
}
```

#### 2. Setup Script Execution
```rust
// In container startup - add these 10 lines
let setup_script = Path::new(".para/docker-env.sh");
if setup_script.exists() {
    let script_content = fs::read_to_string(setup_script)?;
    docker.exec_create(&container_id, CreateExecOptions {
        cmd: Some(vec!["sh", "-c", &script_content]),
        working_dir: Some("/workspace"),
        env: Some(vec!["PARA_WORKSPACE=/workspace"]),
        ..Default::default()
    }).await?;
}
```

### User Guide: Creating Custom Environments

Users create a single file `.para/docker-env.sh` that handles all customization:

```bash
#!/bin/bash
# .para/docker-env.sh - Para Docker Environment Setup

# === ENVIRONMENT VARIABLES ===
# Load from .env file if needed
if [ -f "/workspace/.env" ]; then
    export $(cat /workspace/.env | xargs)
fi

# Set custom variables
export NODE_ENV=development
export DEBUG=true

# === INSTALL DEPENDENCIES ===
# Detect and install based on project files
if [ -f "/workspace/package.json" ]; then
    echo "📦 Installing Node.js dependencies..."
    cd /workspace && npm install
fi

if [ -f "/workspace/requirements.txt" ]; then
    echo "🐍 Installing Python dependencies..."
    pip install -r /workspace/requirements.txt
fi

if [ -f "/workspace/Gemfile" ]; then
    echo "💎 Installing Ruby dependencies..."
    cd /workspace && bundle install
fi

# === CUSTOM TOOLS ===
# Install any additional tools needed
if ! command -v tree &> /dev/null; then
    apt-get update && apt-get install -y tree
fi

# === MOUNT SIMULATION ===
# Link configs from workspace to expected locations
if [ -d "/workspace/.config" ]; then
    ln -sf /workspace/.config ~/.config
fi

if [ -f "/workspace/.ssh/config" ]; then
    mkdir -p ~/.ssh
    cp /workspace/.ssh/config ~/.ssh/
    chmod 600 ~/.ssh/config
fi

# === SERVICES ===
# Start any required services
if [ -f "/workspace/docker-compose.yml" ]; then
    docker-compose up -d
fi

# === FINAL SETUP ===
echo "✅ Para environment ready!"
echo "📁 Workspace: $PARA_WORKSPACE"
echo "🔧 Session: $PARA_SESSION"
```

## User Workflow

### 1. Basic Setup (API Keys Only)
No setup needed - API keys are automatically forwarded:
```bash
export ANTHROPIC_API_KEY=xxx
para start --container
```

### 2. Project Dependencies
Create `.para/docker-env.sh`:
```bash
#!/bin/bash
npm install
npm run build
```

### 3. Complex Environment
Create `.para/docker-env.sh` with:
- Environment variables from .env files
- Tool installation
- Service startup
- Configuration setup

### 4. Custom Docker Image (When Needed)
For cases where the base image needs modification:
```bash
#!/bin/bash
# .para/docker-env.sh

# Check if we need to build custom image
if [ ! -f "/.para-custom-built" ]; then
    echo "Building custom environment..."
    apt-get update
    apt-get install -y python3 nodejs postgresql-client
    
    # Mark as built to avoid rebuilding
    touch /.para-custom-built
fi

# Regular setup continues...
```

## Implementation Steps

### Phase 1: Core Changes (1 hour)
1. Add API key forwarding (5 lines in `DockerService`)
2. Add script execution (10 lines in container startup)
3. Pass `PARA_WORKSPACE` and `PARA_SESSION` to script

### Phase 2: Documentation (30 minutes)
1. Create example `.para/docker-env.sh` templates
2. Add section to README
3. Create cookbook with common scenarios

### Phase 3: Testing (30 minutes)
1. Test API key forwarding
2. Test script execution
3. Test with various project types

## Benefits of This Approach

### Simplicity
- One file to customize everything
- No complex configuration schema
- Uses familiar bash scripting

### Flexibility
- Handle any customization need
- No limits on what users can do
- Easy to debug and modify

### Minimal Code Changes
- ~15 lines of Rust code
- No new dependencies
- No breaking changes

### User-Friendly
- Similar to Dockerfile but simpler
- Can start simple and grow complex
- Easy to share and version control

## Example Templates

### Node.js Development
```bash
#!/bin/bash
source /workspace/.env.local
npm install
npm run dev &
echo "Dev server starting on port 3000..."
```

### Python ML Environment
```bash
#!/bin/bash
pip install -r requirements.txt
jupyter lab --ip=0.0.0.0 --no-browser &
echo "Jupyter available on port 8888"
```

### Full-Stack Development
```bash
#!/bin/bash
# Frontend
cd /workspace/frontend && npm install

# Backend  
cd /workspace/backend && pip install -r requirements.txt

# Database
docker run -d -p 5432:5432 postgres:14

# Start services
cd /workspace && npm run dev
```

## Comparison with Original Plans

| Feature | Original Plan | Simple Plan |
|---------|--------------|-------------|
| Env Variables | Complex .env parsing | Simple script sourcing |
| Custom Images | Dockerfile building | Script-based installation |
| Volume Mounts | Complex mount system | Script-based linking |
| Configuration | JSON schemas | Bash script |
| Code Changes | ~500 lines | ~15 lines |
| User Learning | New concepts | Familiar bash |

## Conclusion

This approach gives users complete control over their Docker environment with minimal implementation complexity. The combination of automatic API key forwarding (for immediate AI agent functionality) and a flexible setup script (for everything else) provides the best balance of simplicity and power.