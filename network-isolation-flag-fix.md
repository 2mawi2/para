# Network Isolation Flag Fix

## Issue
The `--no-network-isolation` flag is redundant since network isolation is OFF by default. This creates confusion and unnecessary complexity.

## Required Changes

### 1. Remove `--no-network-isolation` flag

Remove the flag from:
- `src/cli/parser.rs` - Remove from `StartArgs` and `DispatchArgs` structs
- Update all test files that reference this flag
- Update documentation

### 2. Simplify the logic

In `src/cli/commands/start.rs` and `src/cli/commands/dispatch.rs`, simplify to:

```rust
// Override Docker config with CLI flags
let mut docker_config = config.docker.clone();
if let Some(ref domains) = args.allow_domains {
    // Enable network isolation when --allow-domains is used
    docker_config.network_isolation = true;
    let additional_domains: Vec<String> = domains
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    docker_config.allowed_domains.extend(additional_domains);
}
```

### 3. Update help text

The `--allow-domains` help text should clarify that it enables network isolation:

```rust
#[arg(
    long,
    help = "Enable network isolation and allow access to specified domains (comma-separated)"
)]
pub allow_domains: Option<String>,
```

## Benefits

1. **Clearer UX**: Users only need to think about enabling isolation, not disabling it
2. **Simpler code**: Remove redundant flag handling
3. **Better alignment**: Matches the phased rollout strategy where isolation starts OFF

## Usage Examples

After the fix:
- `para start --container` → No network isolation (default)
- `para start --container --allow-domains ""` → Network isolation with default domains only
- `para start --container --allow-domains "pypi.org"` → Network isolation with default + custom domains

This is much cleaner and more intuitive!