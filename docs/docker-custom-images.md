# Docker Custom Images Guide

This guide explains how to use custom Docker images with para sessions, including configuration options, security considerations, and best practices.

## Overview

Para supports using custom Docker images for container sessions, allowing you to:
- Use pre-built images with your development dependencies
- Share standardized development environments across teams
- Avoid building images locally
- Use images from private registries

## Quick Start

### Using a Custom Image via CLI

```bash
# Start a session with a custom Docker image
para start --container --docker-image node:18-alpine my-node-session

# Dispatch with a custom image
para dispatch --container --docker-image python:3.11 "implement data processing"

# Use a private registry image
para dispatch --container --docker-image mycompany/dev-env:latest "implement feature"
```

### Configuring Default Images

You can set default Docker images in your configuration files:

**Repository-specific config** (`.para/config.json`):
```json
{
  "docker": {
    "default_image": "myproject/dev:latest"
  }
}
```

**Global config** (`~/.config/para/config.json` or platform equivalent):
```json
{
  "docker": {
    "default_image": "ubuntu:22.04"
  }
}
```

## Configuration Priority

Docker images are selected based on the following priority order:

1. **CLI flag** (`--docker-image`) - Highest priority
2. **Repository config** (`.para/config.json` - `docker.default_image`)
3. **Global config** (`~/.config/para/config.json` - `docker.default_image`)
4. **Default** (`para-authenticated:latest`) - Lowest priority

Example:
```bash
# This will use node:18 regardless of config settings
para start --container --docker-image node:18

# This will use the configured default (if any)
para start --container
```

## API Key Forwarding

### Default Behavior

By default, para forwards commonly used API keys to containers:
- `ANTHROPIC_API_KEY`
- `OPENAI_API_KEY`
- `GITHUB_TOKEN`
- `PERPLEXITY_API_KEY`

### Security Warnings

When using custom images with API key forwarding, para displays security warnings:
```
⚠️  API keys: Forwarding to custom image (use --no-forward-keys to disable)
   Security: Only use trusted images when forwarding API keys
```

### Disabling API Key Forwarding

For enhanced security, you can disable API key forwarding:

```bash
# Start without forwarding any API keys
para start --container --docker-image untrusted:latest --no-forward-keys

# Dispatch without API keys
para dispatch --container --docker-image public:latest --no-forward-keys "task"
```

### Configuring Forwarded Keys

You can customize which environment variables are forwarded in your config:

```json
{
  "docker": {
    "default_image": "mycompany/dev:latest",
    "forward_env_keys": [
      "CUSTOM_API_KEY",
      "COMPANY_TOKEN",
      "DATABASE_URL"
    ]
  }
}
```

## Image Validation

Para automatically validates Docker images before creating containers:

1. **Local Check**: First checks if the image exists locally
2. **Auto-Pull**: For custom images not found locally, attempts to pull from registry
3. **Error Handling**: Provides helpful error messages if pull fails

Example output:
```
🐳 Image 'mycompany/dev:latest' not found locally. Attempting to pull...
✅ Successfully pulled image: mycompany/dev:latest
```

## Security Best Practices

### 1. Image Trust

- Only use Docker images from trusted sources
- Verify image signatures when possible
- Use specific tags instead of `latest` for reproducibility
- Consider using image digests for absolute immutability

### 2. API Key Management

- Use `--no-forward-keys` when working with untrusted images
- Configure only necessary keys in `forward_env_keys`
- Rotate API keys regularly
- Never build API keys into Docker images

### 3. Network Isolation

Combine custom images with network isolation for enhanced security:

```bash
# Restricted network access with custom image
para start --container --docker-image node:18 --allow-domains "api.github.com"
```

### 4. Private Registries

For private images, ensure you're logged in before using para:

```bash
# Login to Docker Hub
docker login

# Login to private registry
docker login registry.mycompany.com

# Now use para with private images
para start --container --docker-image registry.mycompany.com/dev:latest
```

## Common Use Cases

### 1. Language-Specific Development

```bash
# Python development
para dispatch --container --docker-image python:3.11-slim "implement ML model"

# Node.js development
para dispatch --container --docker-image node:18-alpine "create REST API"

# Go development
para dispatch --container --docker-image golang:1.21 "build CLI tool"
```

### 2. Full Development Environments

```bash
# Ubuntu with development tools
para start --container --docker-image ubuntu:22.04 dev-session

# Alpine with specific tools
para start --container --docker-image alpine:latest --docker-args "-e TERM=xterm"
```

### 3. Team Standardization

Create a shared configuration:
```json
{
  "docker": {
    "default_image": "myteam/standard-dev:2.0",
    "forward_env_keys": ["GITHUB_TOKEN", "TEAM_API_KEY"]
  }
}
```

Team members can then simply use:
```bash
para start --container
```

## Troubleshooting

### Image Pull Failures

If you see "Failed to pull Docker image", check:
1. Image name is correct (typos are common)
2. You have network connectivity
3. You're logged into the registry (for private images)
4. The image exists in the specified registry

### Missing API Keys in Container

If API keys aren't available in the container:
1. Ensure the keys exist in your host environment
2. Check if `--no-forward-keys` was used
3. Verify the key names in `forward_env_keys` config
4. Check container logs for any startup errors

### Performance Issues

Large images can be slow to pull. Consider:
1. Using smaller base images (alpine variants)
2. Creating custom images with only necessary dependencies
3. Using local image caching strategies
4. Leveraging Docker layer caching

## Example Configurations

### Minimal Python Development
```json
{
  "docker": {
    "default_image": "python:3.11-slim",
    "forward_env_keys": ["OPENAI_API_KEY"]
  }
}
```

### Full-Stack Development
```json
{
  "docker": {
    "default_image": "mycompany/fullstack:latest",
    "forward_env_keys": [
      "GITHUB_TOKEN",
      "DATABASE_URL",
      "REDIS_URL",
      "AWS_ACCESS_KEY_ID",
      "AWS_SECRET_ACCESS_KEY"
    ]
  }
}
```

### Security-Focused Setup
```json
{
  "docker": {
    "default_image": "alpine:latest",
    "forward_env_keys": []
  }
}
```

Then use CLI flags as needed:
```bash
para start --container --allow-domains "" --no-forward-keys
```