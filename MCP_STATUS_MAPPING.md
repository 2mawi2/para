# MCP Status Tool Mapping

This document verifies that MCP tools exactly mirror CLI status commands.

## CLI to MCP Mapping

| CLI Command | MCP Tool Call | Notes |
|-------------|---------------|-------|
| `para status show` | `para_status_show()` | Show all session statuses |
| `para status show --json` | `para_status_show(json: true)` | JSON output for all sessions |
| `para status show session-name` | `para_status_show(session: "session-name")` | Specific session status |
| `para status show session-name --json` | `para_status_show(session: "session-name", json: true)` | Specific session JSON |

## MCP Tool Signature

```typescript
{
  name: "para_status_show",
  inputSchema: {
    type: "object",
    properties: {
      session: {
        type: "string",
        description: "Session name to get status for (optional)"
      },
      json: {
        type: "boolean", 
        description: "Return structured JSON data",
        default: false
      }
    }
  }
}
```

## Implementation

The MCP tool maps directly to CLI arguments:

```typescript
case "para_status_show":
  {
    const cmdArgs = ["status", "show"];
    if ((args as any).session) {
      cmdArgs.push((args as any).session);
    }
    if ((args as any).json) {
      cmdArgs.push("--json");
    }
    result = await runParaCommand(cmdArgs);
  }
  break;
```

## Verification

✅ **Exact CLI Mirroring**: MCP tool parameters map 1:1 to CLI flags
✅ **Argument Order**: Follows CLI help: `para status show [OPTIONS] [SESSION]`  
✅ **Optional Parameters**: Both session and json are optional, matching CLI
✅ **Default Behavior**: No args = show all sessions (same as CLI)
✅ **Error Handling**: Uses same para binary, so error messages match
✅ **Output Format**: Identical output since same underlying command

## Orchestrator Usage Examples

```typescript
// Monitor all agents
await tools.para_status_show()

// Check specific agent with structured data
await tools.para_status_show({
  session: "auth-api-agent", 
  json: true
})

// Get human-readable status for specific agent
await tools.para_status_show({
  session: "frontend-ui-agent"
})
```

This ensures perfect consistency between CLI usage (for agents) and MCP usage (for orchestrators).