# Task: Investigate Configuration Reset Issues

## Overview
Investigate why para configuration values are regularly being reset, losing user customizations. This appears to be happening frequently and is causing workflow disruptions.

## Problem Description
Configuration values in `~/Library/Application Support/para/config.json` are being reset to defaults, losing:
- IDE configuration (switching from claude back to cursor)
- Wrapper settings (enabled/disabled states)
- Custom directory paths
- Git preferences
- Other user customizations

## Primary Investigation Areas

### 1. Test Suite Configuration Interference
**CRITICAL**: Tests should NEVER modify the user's real configuration file. Investigate:

- Search for tests that directly modify configuration files
- Look for tests using real config paths instead of isolated test environments
- Check if any tests are writing to `~/Library/Application Support/para/config.json`
- Verify all tests use `TestEnvironmentGuard` and isolated test configurations
- Find tests that might be calling config setup/reset functions on real config

**Expected Findings**: Tests should only use mock/test configurations, never touch user config.

### 2. Configuration Persistence Issues
Investigate if the configuration system itself has problems:

- Check if config writes are atomic and properly persisted
- Look for race conditions in config read/write operations
- Verify file permissions and write access
- Check if config is being corrupted during writes
- Look for scenarios where config gets overwritten instead of updated

### 3. Auto-Configuration Triggers
Look for code that might automatically reset or reconfigure:

- Auto-detection logic that overwrites existing config
- First-run detection that incorrectly triggers
- Error recovery that resets config to defaults
- Update/migration logic that loses custom settings

### 4. Configuration Loading Issues
Check the configuration loading and saving logic:

- Verify config file is read correctly at startup
- Check for fallback to defaults when config should exist  
- Look for serialization/deserialization problems
- Verify config validation doesn't reset valid values

## Investigation Strategy

### Phase 1: Test Suite Audit
```bash
# Search for configuration-related code in tests
rg -n "config.json|Application Support|PARA_CONFIG" src/
rg -n "Config.*new|Config.*save|Config.*load" src/
rg -n "setup_test|TestEnvironment" src/
```

### Phase 2: Configuration Code Review
1. Review `src/config/` module thoroughly
2. Check all config read/write operations
3. Look for error handling that might reset config
4. Verify atomic writes and proper error recovery

### Phase 3: Real-World Testing
1. Create a test config with custom values
2. Run test suite and check if config changes
3. Run various para commands and monitor config
4. Check config after different error scenarios

### Phase 4: Reproduce the Issue
1. Set up monitoring of config file changes
2. Run common para workflows to trigger resets
3. Identify exact commands or scenarios that cause resets
4. Document reproduction steps

## Specific Areas to Check

### Test Isolation Problems
- Tests calling `Config::new()` without test environment
- Tests using real config directories instead of temp dirs
- Missing `TestEnvironmentGuard` usage in config-related tests
- Tests that don't properly isolate configuration state

### Config System Issues  
- `src/config/mod.rs` - main configuration logic
- Config file write operations - are they atomic?
- Error handling - does it reset config inappropriately?
- Auto-detection logic - does it overwrite existing settings?

## Expected Fixes

### If Tests Are The Problem:
1. Fix all tests to use isolated test configurations
2. Ensure `TestEnvironmentGuard` is used everywhere needed
3. Add safeguards to prevent tests from touching user config
4. Add CI checks to verify test isolation

### If Config System Is The Problem:
1. Fix atomic write operations
2. Improve error handling to preserve existing config
3. Fix auto-detection to not overwrite valid settings
4. Add config backup/recovery mechanisms

## Success Criteria

1. User configuration remains stable across test runs
2. Custom settings persist through para command usage
3. Tests never modify user's real configuration
4. Config resets only happen when explicitly requested
5. Clear error messages when config issues occur

## Testing Requirements

1. Test that running `just test` doesn't change user config
2. Test config persistence across para command usage
3. Test error scenarios don't reset valid config
4. Verify all tests use proper isolation
5. Test concurrent config access doesn't cause corruption

When complete, run: para finish "Fix configuration reset issues and improve test isolation"