# Para Sandboxing

Para provides security features to isolate AI agents and protect against prompt injection attacks. This document outlines the currently implemented sandboxing capabilities.

## Overview

Para implements two main types of sandboxing:

1. **macOS Sandbox-exec** - File system and network restrictions using macOS's built-in sandboxing
2. **Docker Containerization** - Complete isolation through Docker containers

**Note**: This documentation reflects the current implementation as of the codebase review. Some features may have limitations or be under active development.

## macOS Sandboxing

### Platform Support

macOS sandboxing is only available on macOS systems and requires the `sandbox-exec` command.

### Sandbox Profiles

Para uses two sandbox profiles:

#### Standard Profile (`standard`)
- **File Access**: Read access to entire filesystem
- **File Write**: Restricted to specific directories:
  - Session worktree directory
  - Para state directory (`.para/`)
  - Temporary directories (`/tmp`, `/var/folders`)
  - Cache directories
  - Claude configuration (`~/.claude`)
  - Git configuration (`~/.gitconfig`)
- **Network**: Full network access allowed
- **Use Case**: Basic prompt injection protection while maintaining functionality

#### Standard-Proxied Profile (`standard-proxied`)
- **File Access**: Same as Standard profile
- **Network**: Restricted to proxy server only (localhost:8877)
- **Proxy Features**:
  - HTTPS-only connections (port 443)
  - Domain filtering with essential domains always allowed
  - Essential domains: `api.anthropic.com`, `github.com`, `claude.ai`, etc.
- **Use Case**: Network isolation with controlled domain access

### Configuration

#### Global Configuration

```bash
# Enable sandboxing globally
para config set sandbox.enabled true

# Set default profile
para config set sandbox.profile "standard"

# Configure allowed domains for network isolation
para config set sandbox.allowed_domains "example.com,api.service.com"
```

#### Per-Session Flags

```bash
# Enable sandboxing for session
para start --sandbox --prompt "implement feature"

# Disable sandboxing (overrides config)
para start --no-sandbox --prompt "implement feature"

# Use specific profile
para start --sandbox --sandbox-profile "standard-proxied" --prompt "task"

# Network isolation with additional domains
para start --sandbox-no-network --allowed-domains "npmjs.org,pypi.org" --prompt "task"
```

### Command-Line Options

| Flag | Description |
|------|-------------|
| `--sandbox` | Enable sandboxing (overrides config) |
| `--no-sandbox` | Disable sandboxing (overrides config) |
| `--sandbox-profile PROFILE` | Use specific profile (`standard` or `standard-proxied`) |
| `--sandbox-no-network` | Enable network isolation mode |
| `--allowed-domains DOMAINS` | Comma-separated list of additional allowed domains |

## Docker Containerization

### Complete Isolation

Docker containers provide the strongest isolation available on all platforms:

```bash
# Run session in Docker container
para start --container --prompt "implement feature"

# Custom Docker image
para start --container --docker-image "ubuntu:22.04" --prompt "task"

# Network isolation in Docker
para start --container --allow-domains "github.com,api.example.com" --prompt "task"
```

### Configuration

```bash
# Set default Docker image
para config set docker.default_image "mycompany/dev:latest"

# Configure API key forwarding
para config set docker.forward_env_keys "CUSTOM_API_KEY,ANOTHER_KEY"
```

## Security Levels

### Level 1: Basic Protection
- **Configuration**: `--sandbox` (standard profile)
- **Protection**: File write restrictions
- **Network**: Full access
- **Use Case**: Basic prompt injection protection

### Level 2: Network Isolation
- **Configuration**: `--sandbox-no-network`
- **Protection**: File write restrictions + network filtering
- **Network**: Proxy-filtered HTTPS to allowed domains
- **Use Case**: AI agents with controlled internet access

### Level 3: Complete Isolation
- **Configuration**: `--container`
- **Protection**: Full filesystem and network isolation
- **Network**: Configurable with `--allow-domains`
- **Use Case**: Maximum security for untrusted code execution

## Implementation Details

### Dangerous Skip Permissions Flag

The `--dangerously-skip-permissions` flag bypasses IDE permission checks and should only be used:

- For automated scripts
- When dispatching autonomous agents
- In CI/CD environments

**Warning**: This flag does not affect sandboxing security; it only skips user confirmation prompts.

### Network Proxy

The network proxy server (`para proxy`):
- Runs on localhost:8877 by default
- Filters HTTPS connections (port 443 only)
- Blocks non-essential domains
- Supports domain allowlists
- Automatically included in network-isolated sessions

### Session State Persistence

Sandbox settings are persisted per session:
- Resume operations inherit original sandbox settings
- Configuration can be overridden per command
- Settings stored in `.para/state/` directory

## Examples

### Basic AI Agent with File Protection

```bash
para start --sandbox --dangerously-skip-permissions --prompt "implement user authentication"
```

### Network-Isolated Development

```bash
para start --sandbox-no-network --allowed-domains "github.com,npmjs.org" --prompt "update dependencies"
```

### Maximum Security Container

```bash
para start --container --allow-domains "api.anthropic.com" --prompt "analyze untrusted code"
```

### Custom Docker Environment

```bash
para start --container --docker-image "python:3.11" --setup-script "./setup.sh" --prompt "data analysis task"
```

## Troubleshooting

### Common Issues

1. **Sandbox-exec not found**: macOS sandboxing requires `sandbox-exec` command
2. **Network proxy errors**: Check if port 8877 is available
3. **Docker not available**: Ensure Docker is installed and running
4. **Permission denied**: Verify sandbox profile permissions
5. **Temp file access errors**: Some sandbox profile features may have limitations with certain temp file patterns

### Testing Sandbox Functionality

```bash
# Test current sandbox implementation
cargo test sandbox

# Verify Docker integration
cargo test docker
```

### Known Limitations

- Some sandbox profile features may not work correctly with all temp file patterns
- macOS sandbox profiles require sandbox-exec to be available
- Network proxy only supports HTTPS connections (port 443)

## Configuration File Structure

```json
{
  "sandbox": {
    "enabled": true,
    "profile": "standard",
    "allowed_domains": ["example.com", "api.service.com"]
  },
  "docker": {
    "default_image": "para-authenticated:latest",
    "forward_env_keys": ["ANTHROPIC_API_KEY", "GITHUB_TOKEN"]
  }
}
```

## MCP Integration

Para's MCP server exposes sandboxing options through tool parameters:

```typescript
// Basic sandboxing
para_start({
  prompt: "implement feature",
  sandbox: true,
  dangerously_skip_permissions: true
});

// Network isolation
para_start({
  prompt: "fetch data",
  sandbox_no_network: true,
  allow_domains: "api.example.com"
});

// Docker container
para_start({
  prompt: "analyze code",
  container: true,
  docker_image: "ubuntu:22.04"
});
```

## Security Considerations

- **macOS Sandboxing**: Provides file system protection but requires trust in macOS sandbox-exec
- **Docker Containers**: Strongest isolation but requires Docker runtime
- **Network Proxy**: Filters domains but HTTPS traffic is encrypted
- **Session Isolation**: Each session runs in separate worktree
- **Prompt Injection**: File write restrictions prevent most prompt injection attacks

## Future Enhancements

Potential future improvements (not currently implemented):

- Linux namespace-based sandboxing
- Windows sandbox integration
- Fine-grained permission controls
- Process monitoring and limits
- Resource usage constraints