# Docker Feature Gap Summary

## Executive Summary

Para's Docker implementation currently provides basic container support but lacks several important features present in the example project. The most critical missing features are:

1. **Environment Variable Forwarding** - No API keys or terminal settings
2. **Custom Dockerfile Support** - Fixed to pre-built authenticated image only  
3. **Volume Mount Flexibility** - Only workspace directory is mounted
4. **Configuration Persistence** - No Docker settings in config file

## Critical Missing Features

### 1. Environment Variable Forwarding (HIGHEST PRIORITY)
**Impact**: Without API key forwarding, AI tools in containers cannot access external services.

**Current State**:
- Only forwards `PARA_NETWORK_ISOLATION` and `PARA_ALLOWED_DOMAINS`
- No API keys, terminal settings, or proxy configuration

**Needed**:
- Forward API keys (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.)
- Forward terminal environment (TERM, COLORTERM)
- Forward proxy settings for corporate environments
- CLI flag to specify additional env vars

### 2. Custom Dockerfile Support (HIGH PRIORITY)
**Impact**: Users cannot customize container environments for their specific needs.

**Current State**:
- Hardcoded to use `para-authenticated:latest` image
- No ability to add project-specific dependencies

**Needed**:
- Support for `.para/Dockerfile` in project root
- Automatic building when Dockerfile is detected
- Cache built images per project
- Override base image option

### 3. Volume Mount Customization (MEDIUM PRIORITY)
**Impact**: Limited file sharing between host and container.

**Current State**:
- Only mounts working directory as `/workspace`
- No additional mount points

**Needed**:
- Mount user settings directories
- Mount cache/temp directories  
- CLI flag for custom mounts
- Config file mount specifications

### 4. Docker Configuration (MEDIUM PRIORITY)
**Impact**: Users must specify Docker options every time.

**Current State**:
- All Docker settings via CLI only
- No persistence between sessions

**Needed**:
- Docker section in config.json
- Default image, env vars, mounts
- Per-project overrides

## Recommended Implementation Order

### Phase 1: Environment Variables (1-2 days)
Minimal changes to `DockerService::create_container`:
```rust
// Forward essential environment variables
const API_KEYS: &[&str] = &["ANTHROPIC_API_KEY", "OPENAI_API_KEY", ...];
for key in API_KEYS {
    if let Ok(value) = std::env::var(key) {
        docker_args.extend(["-e", &format!("{}={}", key, value)]);
    }
}
```

### Phase 2: Custom Dockerfiles (3-4 days)
1. Check for `.para/Dockerfile`
2. Build image if found
3. Use custom image for session
4. Add `--build` flag to force rebuild

### Phase 3: Configuration Support (2-3 days)
1. Add `DockerConfig` to config structure
2. Update config wizard
3. Apply defaults from config

### Phase 4: Volume Mounts (1-2 days)
1. Add `--mount` CLI flag
2. Parse and validate mount specifications
3. Add to container creation

## Quick Wins

For immediate improvement with minimal code changes:

1. **API Key Forwarding Only** - Add just the essential API keys to get AI tools working
2. **Simple Dockerfile Check** - Just check if `.para/Dockerfile` exists and error with instructions
3. **Documentation** - Document workarounds using `--docker-args`

## Usage Examples After Implementation

```bash
# With environment forwarding
para dispatch --container my-task "Build API endpoint"
# AI tools in container can now access Anthropic API

# With custom Dockerfile
echo "FROM para-authenticated:latest
RUN pip install pandas numpy" > .para/Dockerfile
para dispatch --container --build data-analysis "Analyze CSV data"

# With extra mounts  
para dispatch --container --mount ~/.aws:/root/.aws cloud-deploy "Deploy to AWS"

# With config file
# config.json: { "docker": { "forward_env": ["AWS_*"], "default_image": "my-para:latest" }}
para dispatch --container deploy "Deploy services"
```

## Conclusion

The most impactful change would be implementing environment variable forwarding, as this directly enables AI tools to function properly inside containers. This can be done with minimal code changes and would immediately improve the container experience.

Custom Dockerfile support is the second priority, as it allows users to tailor containers to their project needs. The other features, while useful, can be addressed incrementally.