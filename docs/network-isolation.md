# Network Isolation for Para Docker Containers

## Overview

Para now supports network isolation for Docker containers, providing security by default while maintaining functionality for essential services. This feature is enabled by default and follows security best practices based on Anthropic's reference implementation.

## Features

### Default Network Isolation

- **Enabled by Default**: All new Docker containers run with network isolation unless explicitly disabled
- **Fail-Safe**: If firewall setup fails, the container will not start to prevent insecure operation
- **Default Allowed Domains**:
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

### Basic Usage

```bash
# Start container with default network isolation
para start --container my-session

# Dispatch with container (network isolation enabled by default)
para dispatch --container "implement user authentication"
```

### Adding Custom Allowed Domains

```bash
# Allow additional domains via CLI
para start --container --allow-domains "api.example.com,cdn.example.com" my-session

# Multiple domains in dispatch
para dispatch --container --allow-domains "custom-api.com,internal-service.com" "implement feature"
```

### Disabling Network Isolation (Not Recommended)

```bash
# Disable network isolation (uses host network)
para start --container --no-network-isolation my-session

# Dispatch without network isolation
para dispatch --container --no-network-isolation "implement feature"
```

### Configuration File

Add to your para configuration:

```json
{
  "docker": {
    "enabled": true,
    "mount_workspace": true,
    "network_isolation": true,
    "allowed_domains": [
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
para start --container --allow-domains "localhost:3000,dev.mycompany.com"

# For specific APIs
para dispatch --container --allow-domains "api.stripe.com,api.sendgrid.com" "implement payments"
```

### Debugging Network Issues

To debug network access issues, you can temporarily disable isolation:

```bash
# Temporary disable for debugging (use with caution)
para start --container --no-network-isolation debug-session
```

## Migration from Previous Versions

If you're upgrading from a version without network isolation:

1. **Default Behavior**: Network isolation is now enabled by default
2. **Existing Configs**: Old configurations will automatically use network isolation
3. **Custom Domains**: Add any additional domains you need to the configuration
4. **Backward Compatibility**: Use `--no-network-isolation` if you need the old behavior

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
para dispatch --container \
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