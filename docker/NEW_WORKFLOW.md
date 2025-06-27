# The Correct Docker Workflow for Para

## Build YOUR Image First, Then Authenticate

### Step 1: Create YOUR Dockerfile

Start from any base image and install Claude Code as an npm package:

```dockerfile
FROM ubuntu:22.04  # Or Alpine, Debian, whatever you prefer

# Install Node.js (required for Claude Code)
RUN apt-get update && apt-get install -y curl && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Install Claude Code CLI
RUN npm install -g @anthropic-ai/claude-code@1.0.31

# Install YOUR development tools
RUN apt-get update && apt-get install -y \
    build-essential \
    python3 \
    golang \
    rust \
    # ... whatever you need
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
CMD ["sleep", "infinity"]
```

### Step 2: Build YOUR Image

```bash
docker build -t my-dev-env:latest .
```

### Step 3: Authenticate YOUR Image

Use the helper script to authenticate your custom image:

```bash
./auth-custom-image.sh my-dev-env:latest
```

This creates `my-dev-env-authenticated:latest` with your tools AND Claude authentication.

### Step 4: Use It

```bash
para dispatch feature --container --docker-image my-dev-env-authenticated:latest
```

## Why This Is Better

1. **You control the base image** - Use Ubuntu, Alpine, Debian, whatever
2. **You control the tools** - Install exactly what you need
3. **Claude is just another tool** - Installed via npm like any other package
4. **Authentication is the final step** - Not a prerequisite

## Key Discovery

Claude Code is available as: `@anthropic-ai/claude-code`

This means you can install it in ANY Docker image that has Node.js!