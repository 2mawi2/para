# Gemini CLI Support Integration

THINK HARDER: How can we add Gemini CLI as another AI provider option while reusing the existing provider abstraction and maintaining simple command structure?

## Reference
- Gemini CLI Repository: https://github.com/google-gemini/gemini-cli
- research the README and documentation from this repository for implementation details

## Core Requirement
Para must support dispatching AI-assisted development sessions to Gemini CLI, allowing users to leverage Google's AI capabilities for code generation and workflow automation.

## Observable Behavior
- `para start --ai gemini -p "build API endpoint"` → Creates session with Gemini CLI
- `para config set ai.default_provider gemini` → Sets Gemini as default AI provider
- `para config set ai.gemini.api_key "..."` → Stores Gemini API key securely
- `para start -p "refactor module"` → Uses configured default AI provider
- Gemini execution → Runs with proper working directory and file context

## Implementation Boundaries
- **Must Have**: 
  - Gemini CLI execution with prompt forwarding
  - API key configuration for Gemini/Google Cloud
  - Provider selection via `--ai` flag
  - Environment variable setup for Gemini auth
- **Must NOT Have**: 
  - Gemini-specific command flags or options
  - Custom Gemini authentication flow
  - Media generation or search grounding features
  - Provider-specific UI customizations
- **Can Assume**: 
  - User has Gemini CLI installed via npm/brew
  - User has valid Gemini/Vertex AI API key
  - Gemini CLI handles its own error messages
  - Same prompt format works for all AI providers

## Verification Strategy
1. Write unit test `test_gemini_provider_config()` to verify API key storage
2. Write integration test `test_gemini_session_dispatch()` to verify launch
3. Add test `test_multiple_ai_providers()` to verify provider switching
4. Write test `test_gemini_env_vars()` to verify API key is passed correctly
5. DONE when: `just test` passes and `para start --ai gemini -p "test"` launches Gemini CLI

## Technical Context
- Components: 
  - `src/core/ai_provider.rs` - Extend with Gemini provider implementation
  - `src/config/mod.rs` - Add Gemini-specific configuration (API key)
  - `src/cli/commands/start.rs` - Support "gemini" in `--ai` flag
  - `src/core/session/mod.rs` - Already tracks AI provider from Codex task
- Constraints: 
  - Must set GEMINI_API_KEY or GOOGLE_API_KEY environment variable
  - Cannot modify Gemini CLI's internal behavior
  - Must handle both free and paid API tiers gracefully
- Patterns: 
  - Reuse AI provider abstraction from Codex integration
  - Follow same command execution pattern as other providers
  - Use existing secure credential storage approach

## Development Approach
1. Start with extending AI provider enum to include "gemini" option
2. Write and run unit test for Gemini configuration with API key
3. Only add Gemini command building if provider selection tests pass
4. Stop when integration test confirms Gemini CLI receives prompt and runs in correct directory