# Docker Customization Design

## Overview

This document outlines the design for adding custom Docker image support and environment configuration to para's Docker integration. The goal is to allow users to override the default `para-authenticated:latest` image and configure custom environment variables for their containerized sessions.

## Current State Analysis

### Existing Implementation
- **Docker Image**: Hardcoded to `para-authenticated:latest` 
- **Authentication**: Credentials baked into image at build time
- **Environment Variables**: Limited to `PARA_NETWORK_ISOLATION` and `PARA_ALLOWED_DOMAINS`
- **Docker Args**: Basic support via `--docker-args` CLI flag
- **Configuration**: No Docker-specific settings in config structure

### Key Integration Points
1. `src/core/docker/manager.rs` - Image name retrieval
2. `src/core/docker/service.rs` - Container creation
3. `src/cli/parser.rs` - Command-line argument parsing
4. `src/config/mod.rs` - Configuration structure

## Proposed Solutions

### Solution 1: Configuration-Based Customization

Add Docker settings to para's configuration file:

```json
{
  "docker": {
    "default_image": "mycompany/dev-env:latest",
    "default_env_file": ".env.docker",
    "allow_custom_images": true
  }
}
```

**Pros:**
- Clean, declarative configuration
- Per-project customization via `.para/config.json`
- Version control friendly
- Persistent settings

**Cons:**
- Requires config file modifications
- Less flexible for one-off uses

### Solution 2: Enhanced CLI Arguments

Extend CLI with specific Docker flags:

```bash
para start --container --docker-image custom:latest --env-file .env
```

**Pros:**
- No configuration changes needed
- Maximum flexibility for one-off sessions
- Builds on existing `--docker-args` pattern

**Cons:**
- Verbose for complex setups
- No persistence between sessions

### Solution 3: Docker Profiles System

Introduce reusable Docker configuration profiles:

```json
{
  "docker_profiles": {
    "nodejs-dev": {
      "image": "node:18-alpine",
      "env_file": ".env.development",
      "volumes": ["./config:/app/config:ro"],
      "docker_args": ["-m", "4g"]
    }
  }
}
```

**Pros:**
- Reusable configurations
- Team-sharable profiles
- Supports complex setups

**Cons:**
- Additional abstraction layer
- More complex implementation

### Solution 4: Hybrid Approach (Recommended)

Combine all approaches for maximum flexibility:

## Recommended Implementation

### 1. Configuration Structure Extension

```rust
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    // ... existing fields ...
    pub docker: Option<DockerConfig>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DockerConfig {
    pub default_image: Option<String>,
    pub default_env_file: Option<String>,
    pub profiles: HashMap<String, DockerProfile>,
    pub allow_custom_images: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DockerProfile {
    pub image: String,
    pub env_file: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub docker_args: Vec<String>,
    pub volumes: Vec<String>,
}
```

### 2. CLI Enhancement

Add new Docker-specific options:

```rust
#[derive(Args, Debug, Clone)]
pub struct DockerOptions {
    /// Use custom Docker image
    #[arg(long = "docker-image")]
    pub docker_image: Option<String>,
    
    /// Load environment from file
    #[arg(long = "env-file")]
    pub env_file: Option<PathBuf>,
    
    /// Use Docker profile from config
    #[arg(long = "docker-profile")]
    pub docker_profile: Option<String>,
    
    /// Set individual environment variables
    #[arg(long = "env", short = 'e')]
    pub env_vars: Vec<String>,
}
```

### 3. Environment Variable Handling

#### File Support
- Support `.env` files in project root (auto-detected)
- Support explicit paths via `--env-file`
- Parse standard `.env` format
- Merge with authentication environment

#### Variable Priority (highest to lowest)
1. CLI `-e` flags
2. `--env-file` specified file
3. Profile environment variables
4. Project `.env` file (if exists)
5. Default authenticated environment

### 4. Image Selection Priority

1. CLI `--docker-image` flag (highest priority)
2. Docker profile image
3. Config `default_image`
4. Default to `para-authenticated:latest`

### 5. Security Considerations

- Validate image existence before use
- Warn when using non-authenticated images
- Preserve network isolation capabilities
- Secure handling of environment files
- Optional image allowlist in config

## Usage Examples

### Basic Custom Image
```bash
para start --container --docker-image mycompany/dev-env:latest
```

### With Environment File
```bash
para start --container --env-file .env.development
```

### Using Docker Profile
```bash
para start --container --docker-profile nodejs-dev
```

### Complex Example
```bash
para start --container \
  --docker-image custom-image:v2 \
  --env-file .env.production \
  -e "DEBUG=true" \
  -e "API_KEY=xyz" \
  --docker-args="-m 4g --cpus 2" \
  "implement authentication feature"
```

### Configuration Example
```json
{
  "docker": {
    "default_image": "para-authenticated:latest",
    "allow_custom_images": true,
    "profiles": {
      "nodejs": {
        "image": "node:18-alpine",
        "env_file": ".env.node",
        "env_vars": {
          "NODE_ENV": "development"
        },
        "volumes": ["./config:/app/config:ro"],
        "docker_args": ["-m", "2g"]
      },
      "python-ml": {
        "image": "python:3.11-cuda",
        "env_file": ".env.ml",
        "docker_args": ["--gpus", "all"]
      }
    }
  }
}
```

## Implementation Phases

### Phase 1: Basic Custom Image Support
- Add `--docker-image` CLI flag
- Modify `get_docker_image()` to check CLI args
- Add image validation
- Update documentation

### Phase 2: Environment File Support
- Add `--env-file` CLI flag
- Implement `.env` file parser
- Auto-detect project `.env` files
- Merge environment variables during container creation

### Phase 3: Configuration Integration
- Extend Config struct with DockerConfig
- Update configuration wizard
- Implement config-based defaults
- Add config validation

### Phase 4: Docker Profiles
- Implement profile system
- Add `--docker-profile` CLI flag
- Support profile inheritance
- Add profile management commands

## Benefits

1. **Flexibility**: Supports both one-off and persistent configurations
2. **Backward Compatibility**: Existing workflows continue unchanged
3. **Progressive Enhancement**: Can implement in phases
4. **Team Collaboration**: Profiles shareable via version control
5. **Security**: Maintains authentication and isolation features
6. **User Experience**: Simple defaults with powerful customization

## Migration Path

1. Existing users continue using `para-authenticated:latest` by default
2. Power users can immediately use `--docker-image` flag
3. Teams can gradually adopt profiles for standardization
4. Documentation guides users through customization options

## Future Considerations

- Docker Compose support for multi-container scenarios
- Container templates with pre-configured development environments
- Integration with container registries for private images
- Resource limit profiles (memory, CPU, GPU)
- Development environment inheritance chains