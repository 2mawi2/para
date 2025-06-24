# Network Isolation Branch - Required Fixes

## Overview
This document outlines the required fixes for the `network-isolation` branch before it can be merged into `feature/docker-poc`.

## Required Fixes

### 1. Fix Default Network Isolation Setting

**Issue**: Documentation states network isolation is "OFF by default", but the implementation defaults to `true`.

**Fix Required**:
- In `docker/secure-entrypoint.sh`, change line:
  ```bash
  NETWORK_ISOLATION="${PARA_NETWORK_ISOLATION:-true}"
  ```
  To:
  ```bash
  NETWORK_ISOLATION="${PARA_NETWORK_ISOLATION:-false}"
  ```

### 2. Remove Hardcoded Domains

**Issue**: The `docker/init-firewall.sh` script contains hardcoded domains that aren't documented.

**Fix Required**:
- Remove hardcoded domains `sentry.io` and `statsig.anthropic.com` from `docker/init-firewall.sh`
- These should only be added if explicitly specified via `--allow-domains` flag

### 3. Add Documentation for Common Use Cases

**Issue**: Missing documentation for common development scenarios.

**Fix Required**:
Add the following examples to `docs/network-isolation.md` in the appropriate section:

```markdown
### Common Development Scenarios

#### NPM Package Installation
When using npm packages that require network access during installation:
```bash
# Allow npm registry access
para start --container --allow-domains "registry.npmjs.org,cdn.jsdelivr.net" my-session
```

#### Python Package Installation
For Python development with pip:
```bash
# Allow PyPI access
para start --container --allow-domains "pypi.org,files.pythonhosted.org" my-session
```

#### Custom API Development
When developing against custom APIs:
```bash
# Allow your API endpoints
para start --container --allow-domains "api.mycompany.com,staging-api.mycompany.com" my-session
```
```

### 4. Fix Error Handling Consistency

**Issue**: Different error handling for GitHub IP fetching vs domain resolution failures.

**Fix Required**:
In `docker/init-firewall.sh`:
- Make domain resolution failures exit with error (same as GitHub IP fetch failures)
- Change the domain resolution error handling to:
  ```bash
  if ! resolve_domain "$domain"; then
      echo "ERROR: Failed to resolve domain: $domain" >&2
      exit 1
  fi
  ```
- This ensures consistent fail-safe behavior - if any allowed domain cannot be resolved, the container should not start

### 5. Add Capability Check

**Issue**: No explicit verification of NET_ADMIN/NET_RAW capabilities before attempting iptables configuration.

**Fix Required**:
In `docker/secure-entrypoint.sh`, add capability check before network isolation setup:
```bash
# Check for required capabilities
if [ "$NETWORK_ISOLATION" = "true" ]; then
    # Check if we have the required capabilities
    if ! capsh --print | grep -q "cap_net_admin" || ! capsh --print | grep -q "cap_net_raw"; then
        echo "ERROR: Network isolation requires NET_ADMIN and NET_RAW capabilities" >&2
        echo "Please ensure the container is running with: --cap-add=NET_ADMIN --cap-add=NET_RAW" >&2
        exit 1
    fi
    
    # Existing network isolation setup code...
fi
```

Note: If `capsh` is not available in the container, use an alternative method:
```bash
# Alternative: Check if we can actually use iptables
if ! iptables -L >/dev/null 2>&1; then
    echo "ERROR: Cannot access iptables. Network isolation requires NET_ADMIN and NET_RAW capabilities" >&2
    echo "Please ensure the container is running with: --cap-add=NET_ADMIN --cap-add=NET_RAW" >&2
    exit 1
fi
```

## Testing Checklist

After implementing these fixes, verify:

1. [ ] Default behavior: `para start --container` starts WITHOUT network isolation
2. [ ] Warning message appears when network isolation is OFF
3. [ ] Network isolation can be enabled with `--allow-domains ""`
4. [ ] No hardcoded domains are allowed unless explicitly specified
5. [ ] Container fails to start if:
   - Required capabilities are missing
   - Any allowed domain cannot be resolved
   - GitHub IP ranges cannot be fetched (when github.com is allowed)
6. [ ] Documentation examples work for npm and pip package installation

## Implementation Notes

- Maintain fail-safe approach: when in doubt, fail closed (deny network access)
- All error messages should be clear and actionable
- Keep consistent with the phased rollout strategy outlined in documentation