# Para Network Sandboxing Test Scenarios

## Test Results Summary

✅ **All sandbox-related tests pass successfully** (71 tests)

### Implemented Features:

1. **Basic File Sandboxing** (`--sandbox`)
   - Uses standard sandbox profiles
   - Restricts file system access
   - Compatible with all para start methods

2. **Network Sandboxing** (`--sandbox-no-network`)
   - Blocks all network access by default
   - Runs HTTP proxy outside sandbox
   - Command runs inside sandbox with proxy env vars
   - Always allows essential Claude domains

3. **Custom Domain Allowlist** (`--allowed-domains`)
   - Additional domains can be allowed
   - Works with network sandboxing
   - Supports subdomain matching

4. **Architecture Changes**:
   - Created `launcher_v2.rs` with cleaner separation
   - Proxy runs outside sandbox (fixes circular dependency)
   - Wrapper scripts handle complex setups
   - SandboxResolver handles network sandbox flag

### Test Coverage:

- ✅ CLI flag parsing and validation
- ✅ Sandbox profile extraction and validation
- ✅ Network proxy server functionality
- ✅ Domain filtering and allowlisting
- ✅ Wrapper script generation
- ✅ Integration with all start methods
- ✅ Error handling and edge cases

### Usage Examples:

```bash
# Basic file sandboxing
para start --sandbox

# Network sandboxing (blocks all except Claude)
para start --sandbox-no-network

# Network sandboxing with additional domains
para start --sandbox-no-network --allowed-domains "github.com,*.openai.com"

# Works with all start methods
para start my-feature --sandbox-no-network
para start -p "implement feature" --sandbox-no-network
para start --file prompt.txt --sandbox-no-network
```

### Key Implementation Details:

1. **Proxy Server** (`src/core/sandbox/proxy.rs`)
   - Synchronous HTTP CONNECT proxy
   - No external dependencies (no tokio/async)
   - Filters HTTPS connections by domain

2. **Sandbox Profiles** (`src/core/sandbox/profiles/`)
   - `standard-proxied.sb` - Network-restricted profile
   - Allows proxy connection on localhost
   - Blocks all other network access

3. **Launcher V2** (`src/core/sandbox/launcher_v2.rs`)
   - Returns structured `SandboxedCommand`
   - Indicates if wrapper script needed
   - Provides proxy port information

4. **Integration Points**:
   - `start.rs` - CLI argument handling
   - `dispatch.rs` - Agent dispatch support
   - `ide.rs` - IDE launch integration
   - `claude_launcher.rs` - Claude-specific handling