# Docker Feature Analysis for Para

## Executive Summary

This analysis compares the Docker isolation features in Para with a reference implementation (Gemini CLI). Para currently has basic Docker support but lacks several critical features that would enable more robust containerized development workflows.

## Current Para Docker Implementation

### What Para Has
- ✅ Basic Docker container creation and execution
- ✅ Network isolation support
- ✅ Docker authentication integration
- ✅ Container lifecycle management (start, stop, cleanup)
- ✅ Basic volume mounting (current directory only)

### What Para Is Missing

#### 1. **Environment Variable Forwarding** (Critical)
The most significant gap is the lack of environment variable forwarding. This prevents AI tools and development processes inside containers from accessing:
- API keys (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.)
- Terminal settings (TERM, SHELL)
- Proxy configurations (HTTP_PROXY, HTTPS_PROXY)
- Custom environment variables

**Impact**: AI agents cannot function properly without API keys.

#### 2. **Custom Docker Images** (High Priority)
Para uses a fixed `para-authenticated:latest` image with no support for:
- Custom Dockerfiles (e.g., `.para/Dockerfile`)
- Build-time customization
- Project-specific dependencies
- Different base images per project

**Impact**: Cannot install project-specific tools or dependencies.

#### 3. **Flexible Volume Mounts** (Medium Priority)
Currently only mounts the current working directory. Missing:
- Home directory access for config files
- Temporary directory access
- Custom mount points via configuration
- Read-only mount options

**Impact**: Limited access to configuration files and system resources.

#### 4. **Configuration Persistence** (Medium Priority)
No Docker-specific configuration in `config.json`:
- Cannot set default Docker options
- No project-specific Docker settings
- No image override configuration

**Impact**: Must specify Docker options on every command.

## Recommended Implementation Plan

### Phase 1: Environment Variable Forwarding (1-2 days)
Add essential environment variable forwarding to `DockerService::create_container()`:

```rust
// In src/core/docker.rs
let env_vars = vec![
    format!("ANTHROPIC_API_KEY={}", std::env::var("ANTHROPIC_API_KEY").unwrap_or_default()),
    format!("OPENAI_API_KEY={}", std::env::var("OPENAI_API_KEY").unwrap_or_default()),
    format!("TERM={}", std::env::var("TERM").unwrap_or_default()),
    // Add more as needed
];
```

### Phase 2: Custom Dockerfile Support (2-3 days)
1. Check for `.para/Dockerfile` in project root
2. Build custom image if present
3. Use custom image for containers
4. Cache built images

### Phase 3: Docker Configuration (1 day)
Add Docker section to config.json:
```json
{
  "docker": {
    "enabled": true,
    "default_image": "para-authenticated:latest",
    "environment": ["ANTHROPIC_API_KEY", "OPENAI_API_KEY"],
    "mounts": ["/tmp", "~/.config"],
    "build_cache": true
  }
}
```

### Phase 4: Enhanced Volume Mounts (1 day)
- Add configurable mount points
- Support read-only mounts
- Handle cross-platform path differences

## Quick Wins

The fastest improvement with the highest impact would be adding environment variable forwarding. This can be implemented in under 50 lines of code and would immediately enable AI tools to function inside containers.

## Code Changes Required

### 1. DockerService Enhancement (src/core/docker.rs)
- Modify `create_container()` to accept environment variables
- Add `build_custom_image()` method
- Enhance volume mount logic

### 2. Configuration Updates (src/config/mod.rs)
- Add `DockerConfig` struct
- Update `Config` to include Docker settings
- Add validation for Docker options

### 3. CLI Integration (src/cli/commands/dispatch.rs)
- Add `--docker-env` flag for custom environment variables
- Add `--docker-build` flag to trigger custom builds
- Update help text with Docker options

## Comparison Table

| Feature | Para | Reference (Gemini CLI) |
|---------|------|------------------------|
| Basic Container Support | ✅ | ✅ |
| Environment Variables | ❌ | ✅ |
| Custom Dockerfiles | ❌ | ✅ |
| Flexible Mounts | ❌ | ✅ |
| Config Persistence | ❌ | ✅ |
| Build-time Customization | ❌ | ✅ |
| Runtime Customization | ❌ | ✅ |
| Cross-platform Support | ✅ | ✅ |

## Conclusion

Para has a solid foundation for Docker support but lacks critical features for production use. The highest priority should be implementing environment variable forwarding, which would immediately enable AI agents to function properly inside containers. The subsequent phases would add flexibility and customization options that make Para more suitable for diverse development environments.

## Next Steps

1. Implement environment variable forwarding (Phase 1)
2. Test with AI agents to ensure API access works
3. Gather feedback on additional environment variables needed
4. Proceed with custom Dockerfile support (Phase 2)
5. Add configuration persistence for user convenience