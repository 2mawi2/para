# Codex Support Integration

THINK HARDER: How can we integrate Codex as an alternative AI provider for para sessions while maintaining the existing architecture and command structure?

## Reference
- Codex Repository: https://github.com/openai/codex
- research the README and documentation from this repository for implementation details

## Core Requirement
Para must support dispatching AI-assisted development sessions to Codex in addition to Claude Code, allowing users to choose their preferred AI coding assistant.

## Observable Behavior
- `para start --ai codex -p "implement feature"` → Creates session with Codex as AI agent
- `para config set ai.default_provider codex` → Sets Codex as default AI provider
- `para start -p "fix bug"` → Uses default AI provider (Codex if configured)
- `para list` → Shows sessions with AI provider type (e.g., "codex", "claude")
- Codex command execution → Runs in para worktree directory with proper context

## Implementation Boundaries
- **Must Have**: 
  - Basic Codex execution with prompt forwarding
  - Configuration option for AI provider selection
  - API key management for OpenAI/Codex
  - Session tracking shows which AI provider is used
- **Must NOT Have**: 
  - Complex Codex configuration management
  - Custom Codex sandbox modifications
  - Multiple AI providers in single session
  - Codex-specific UI features
- **Can Assume**: 
  - User has Codex installed globally
  - User has valid OpenAI API key
  - Default to Claude Code if no provider specified
  - Codex handles its own error reporting

## Verification Strategy
1. Write unit test `test_codex_provider_config()` to verify provider configuration
2. Write integration test `test_codex_session_creation()` to verify session dispatch
3. Add test `test_ai_provider_detection()` to verify correct provider is used
4. Run `just test` to ensure all existing tests pass
5. DONE when: All tests pass and `para start --ai codex -p "test"` successfully launches Codex

## Technical Context
- Components: 
  - `src/config/mod.rs` - Add AI provider configuration
  - `src/core/ai_provider.rs` - New module for AI provider abstraction
  - `src/cli/commands/start.rs` - Add `--ai` flag for provider selection
  - `src/core/session/mod.rs` - Track AI provider in session metadata
- Constraints: 
  - Must maintain backward compatibility with existing Claude Code sessions
  - Cannot modify Codex's internal behavior
  - Must respect existing para architecture
- Patterns: 
  - Follow existing IDE integration pattern in `src/core/ide.rs`
  - Use similar command execution as current Claude Code integration
  - Maintain existing session state structure

## Development Approach
1. Start with adding `ai_provider` field to Config struct with "claude" default
2. Write and run unit test for configuration serialization/deserialization
3. Only add command execution if configuration tests pass
4. Stop when integration test shows Codex launches with correct prompt and working directory