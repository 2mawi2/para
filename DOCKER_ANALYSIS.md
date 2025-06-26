# Docker Features Analysis: Para vs Example Project

## Current Para Docker Implementation

### 1. Docker Container Support
- **Basic Container Creation**: Creates containers with `para-<session-name>` naming
- **Volume Mounting**: Mounts working directory as `/workspace`
- **Network Isolation**: Optional network isolation with allowed domains
- **Authentication**: Pre-built authenticated image (`para-authenticated:latest`)
- **Container Pool**: Management of multiple containers
- **IDE Integration**: Launch IDE connected to container

### 2. Current Docker Arguments
- `--container` / `-c`: Run session in Docker container
- `--allow-domains`: Enable network isolation with allowed domains
- `--docker-args`: Pass additional Docker arguments

### 3. Image Management
- **Fixed Image**: Uses `para-authenticated:latest` image
- **Authentication Flow**: Interactive auth setup via `para auth setup`
- **Auth Volume**: Persistent authentication storage

## Missing Features Compared to Example Project

### 1. Custom Docker Images ❌
**Example Project**: 
- Supports custom Dockerfiles via `.gemini/sandbox.Dockerfile`
- `BUILD_SANDBOX=1` triggers custom image builds
- Can override base image

**Para**: 
- Fixed to `para-authenticated:latest` image only
- No support for custom Dockerfiles
- No build-time customization

### 2. Environment Variable Forwarding ❌
**Example Project**:
- Selectively forwards API keys (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.)
- Forwards TERM for terminal support
- Forwards proxy settings (HTTP_PROXY, HTTPS_PROXY, NO_PROXY)
- Runtime customization via `SANDBOX_ENV`

**Para**:
- Only sets `PARA_NETWORK_ISOLATION` and `PARA_ALLOWED_DOMAINS`
- No API key forwarding
- No proxy support
- No terminal environment forwarding

### 3. Volume Mount Customization ❌
**Example Project**:
- Mounts multiple directories (project, settings, temp)
- Runtime customization via `SANDBOX_MOUNTS`
- Configurable mount paths

**Para**:
- Only mounts working directory as `/workspace`
- No additional volume mount support
- No runtime mount customization

### 4. Docker Configuration in Config File ❌
**Example Project**:
- Docker settings in configuration file
- Configurable base image
- Persistent Docker preferences

**Para**:
- No Docker configuration in config file
- All Docker settings via CLI arguments only
- No persistent Docker preferences

### 5. Build Context Support ❌
**Example Project**:
- Can build images with context from project directory
- Supports multi-stage builds
- Custom build arguments

**Para**:
- No build support
- Requires pre-built images

## Implementation Priority

### High Priority
1. **Environment Variable Forwarding**
   - Essential for AI model access (API keys)
   - Terminal support (TERM)
   - Proxy configuration

2. **Custom Dockerfile Support**
   - Allow `.para/Dockerfile` or similar
   - Build custom images per project
   - Override base image

### Medium Priority
3. **Volume Mount Customization**
   - Additional mount points
   - Settings/config directories
   - Shared cache directories

4. **Docker Configuration**
   - Add Docker section to config file
   - Default image settings
   - Default environment variables

### Low Priority
5. **Advanced Features**
   - Multi-stage build support
   - Build caching optimization
   - Container resource limits

## Proposed Implementation Plan

### Phase 1: Environment Variables
1. Add env forwarding to `DockerService::create_container`
2. Create whitelist of safe environment variables
3. Add `--env` flag for custom env vars

### Phase 2: Custom Images
1. Check for `.para/Dockerfile` in project root
2. Build image if Dockerfile exists
3. Add `--build` flag to force rebuild
4. Cache built images by project

### Phase 3: Configuration
1. Add `docker` section to `Config` struct
2. Support default image, env vars, mounts
3. Allow per-project overrides

### Phase 4: Volume Mounts
1. Add `--mount` flag for additional mounts
2. Support settings and cache directories
3. Runtime mount configuration

## Code Locations to Modify

1. **DockerService** (`src/core/docker/service.rs`)
   - Add environment variable forwarding
   - Support custom image names
   - Additional volume mounts

2. **DockerManager** (`src/core/docker/manager.rs`)
   - Image building logic
   - Dockerfile detection

3. **Config** (`src/config/mod.rs`)
   - Add Docker configuration section

4. **CLI Parser** (`src/cli/parser.rs`)
   - Add new Docker-related flags
   - Environment and mount options

5. **Dispatch Command** (`src/cli/commands/dispatch.rs`)
   - Handle new Docker options
   - Build custom images