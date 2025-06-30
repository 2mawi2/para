# Sandbox Integration Fixes

The review identified several critical issues that need to be addressed:

## 1. Configuration Migration
Add a configuration migration system to handle the new `sandbox` field. Create a migration function that:
- Detects old config format without `sandbox` field
- Adds default `sandbox: false` for existing configurations
- Preserves all other settings

## 2. Platform-specific Code Improvements
Enhance the `is_sandbox_available()` function to:
- Return false gracefully on non-macOS platforms
- Add proper error messages when sandbox isn't available
- Ensure the feature degrades gracefully without breaking functionality

## 3. Security Hardening
- Validate sandbox profile names against a whitelist before use
- Add proper validation of profile content before extraction
- Ensure temporary files are properly cleaned up even on errors
- Add bounds checking for profile extraction

## 4. Error Handling Improvements
- Add validation in `wrap_with_sandbox` for profile name format
- Handle cases where profile extraction fails
- Provide clear error messages for sandbox initialization failures
- Add proper error propagation throughout the sandbox module

## 5. Test Coverage Enhancement
Add tests for:
- Configuration migration from old to new format
- Error paths in profile validation
- Temporary file cleanup on failures
- Platform detection and graceful degradation

## 6. Remove Documentation File
Remove the `docs/sandboxing.md` file as it was created without explicit user request, violating project guidelines.

After implementing all fixes:
1. Commit all changes: `git add . && git commit -m 'Fix sandbox integration issues: config migration, security hardening, error handling'`
2. Verify build works: `just build`
3. Run: `para finish 'Fix sandbox integration issues'`