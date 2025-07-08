# Network Isolation for Para Docker Containers

## Overview

Para supports network isolation for Docker containers, providing enhanced security while maintaining functionality for essential services. This feature follows security best practices based on Anthropic's reference implementation. Network isolation is currently **OFF by default** for backward compatibility, with a phased rollout strategy planned.

## Features

### Network Isolation Status

- **Currently OFF by Default**: For backward compatibility with existing workflows
- **Warning Display**: When network isolation is disabled, you'll see: `⚠️  Network isolation: OFF (use --allow-domains to enable)`
- **Easy Enable**: Simply use `--allow-domains` flag to enable isolation with default or custom domains
- **Fail-Safe**: When enabled, if firewall setup fails, the container will not start to prevent insecure operation
- **Default Allowed Domains** (when enabled):
  - `api.anthropic.com` (Claude API)
  - `github.com` (Git operations)
  - `api.github.com` (GitHub API)
  - `registry.npmjs.org` (npm package registry)

### Security Implementation

Based on Anthropic's reference implementation using:
- **iptables** for network filtering
- **ipset** for efficient domain-based access control
- **Default deny** policy with explicit allow rules
- **GitHub API integration** for dynamic IP range updates
- **Domain resolution** with CNAME support

## Usage

### Basic Usage (Without Network Isolation)

```bash
# Start container without network isolation (current default)
para start --container my-session
# ⚠️  Network isolation: OFF (use --allow-domains to enable)

# Start AI-assisted session with container (no network isolation by default)
para start -p "implement user authentication" --container
# ⚠️  Network isolation: OFF (use --allow-domains to enable)
```

### Enabling Network Isolation (Recommended)

```bash
# Enable network isolation with default allowed domains
para start my-session --container --allow-domains ""

# Enable with additional custom domains
para start my-session --container --allow-domains "api.example.com,cdn.example.com"

# Start AI-assisted session with network isolation enabled
para start -p "implement user authentication" --container --allow-domains ""
```

### Custom Allowed Domains

```bash
# Add specific domains for your use case
para start payment-feature --container --allow-domains "api.stripe.com,api.sendgrid.com"

# Multiple domains in AI-assisted session
para start -p "implement feature" --container --allow-domains "custom-api.com,internal-service.com"
```

### Explicitly Disabling Network Isolation

```bash
# Explicitly disable network isolation (same as current default)
para start --container --no-network-isolation my-session

# Note: This is currently equivalent to not using --allow-domains
```

### Common Development Scenarios

#### NPM Package Installation
When using npm packages that require network access during installation:
```bash
# Allow npm registry access
para start my-session --container --allow-domains "registry.npmjs.org,cdn.jsdelivr.net"
```

#### Python Package Installation
For Python development with pip:
```bash
# Allow PyPI access
para start my-session --container --allow-domains "pypi.org,files.pythonhosted.org"
```

#### Custom API Development
When developing against custom APIs:
```bash
# Allow your API endpoints
para start my-session --container --allow-domains "api.mycompany.com,staging-api.mycompany.com"
```

### Configuration File

Add to your para configuration:

```json
{
  "docker": {
    "enabled": true,
    "mount_workspace": true,
    "network_isolation": false,  // Currently defaults to false
    "allowed_domains": [
      "my-api.com",
      "internal-service.com"
    ]
  }
}
```

To enable network isolation by default in your configuration:

```json
{
  "docker": {
    "enabled": true,
    "mount_workspace": true,
    "network_isolation": true,   // Enable by default
    "allowed_domains": [
      // Add any additional domains beyond the defaults
      "my-api.com",
      "internal-service.com"
    ]
  }
}
```

## Security Details

### Firewall Rules

The network isolation implements the following security measures:

1. **Default Deny**: All network traffic is blocked by default
2. **Essential Services**: DNS, localhost, and SSH are always allowed
3. **Dynamic GitHub IPs**: Fetches and allows current GitHub IP ranges
4. **Domain Resolution**: Resolves allowed domains to IPs and allows access
5. **Host Network Access**: Allows communication with the Docker host network

### Verification

The firewall setup includes automatic verification:
- **Negative Test**: Ensures blocked domains (like `example.com`) are inaccessible
- **Positive Test**: Verifies allowed domains (like `api.anthropic.com`) are accessible
- **Fail-Safe**: Container startup fails if verification fails

## Troubleshooting

### Container Fails to Start

If you see firewall verification errors:

1. **Check DNS Resolution**: Ensure your system can resolve domain names
2. **Docker Capabilities**: The container needs `NET_ADMIN` and `NET_RAW` capabilities (automatically added)
3. **Network Connectivity**: Ensure your host has internet access

### Adding Domains for Special Use Cases

```bash
# For internal development
para start my-session --container --allow-domains "localhost:3000,dev.mycompany.com"

# For specific APIs
para start -p "implement payments" --container --allow-domains "api.stripe.com,api.sendgrid.com"
```

### Debugging Network Issues

To debug network access issues, you can temporarily disable isolation:

```bash
# Temporary disable for debugging (use with caution)
para start --container --no-network-isolation debug-session
```

## Migration and Future Plans

### Current Status (Phased Rollout)

1. **Default OFF**: Network isolation is currently OFF by default for backward compatibility
2. **Warning System**: Users see a warning when network isolation is disabled
3. **Easy Opt-in**: Use `--allow-domains` flag to enable network isolation
4. **Configuration**: Can be enabled by default in your configuration file

### Future Plans

1. **Phase 1 (Current)**: Network isolation OFF by default with warnings
2. **Phase 2 (Future)**: Network isolation ON by default with opt-out option
3. **Phase 3 (Later)**: Network isolation always ON for maximum security

### Migration Tips

- **Test with isolation**: Try `--allow-domains ""` to test your workflows with isolation
- **Identify required domains**: Note any additional domains your workflows need
- **Update configuration**: Add `network_isolation: true` to enable by default
- **Gradual adoption**: Enable for specific sessions first before making it default

## Security Considerations

### Recommended Practices

- **Keep isolation enabled**: Only disable for debugging or specific use cases
- **Minimal domains**: Only add domains that are absolutely necessary
- **Regular review**: Periodically review your allowed domains list
- **Use HTTPS**: The firewall allows both HTTP and HTTPS, but prefer HTTPS

### Understanding the Firewall

The network isolation:
- **Does NOT** protect against malicious code execution
- **DOES** limit network access to approved domains
- **Prevents** accidental data exfiltration to unknown domains
- **Allows** necessary development and AI assistant functionality

## Examples

### Basic Development Workflow

```bash
# Start a secure container session
para start --container secure-development

# The container will have access to:
# - Claude API for AI assistance
# - GitHub for version control
# - npm registry for package installation
# - No other external services
```

### Custom API Integration

```bash
# Working with external APIs
para start --container \
  --allow-domains "api.stripe.com,api.sendgrid.com" \
  "Implement payment processing with Stripe and email notifications with SendGrid"
```

### Internal Development

```bash
# For internal company services
para start --container \
  --allow-domains "api.internal.company.com,auth.company.com" \
  internal-feature
```

## Technical Details

### Container Requirements

The network isolation requires:
- Ubuntu 22.04 base image (or compatible)
- `iptables`, `ipset`, `dnsutils`, `iproute2`, `aggregate`, `jq` packages
- `NET_ADMIN` and `NET_RAW` capabilities (automatically added)

### Script Locations

- **Firewall Script**: `/usr/local/bin/init-firewall.sh`
- **Secure Entrypoint**: `/usr/local/bin/secure-entrypoint.sh`
- **Environment Variables**:
  - `PARA_NETWORK_ISOLATION`: Enable/disable isolation
  - `PARA_ALLOWED_DOMAINS`: Comma-separated list of allowed domains

For more information, see the source code in `src/core/docker/` and `docker/` directories.