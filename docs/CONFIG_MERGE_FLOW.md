# Configuration Merge Flow Under the Hood

## Overview

Para uses a hierarchical configuration system where **both** user (global) and project configurations are **merged** together, with project settings taking precedence for specific fields.

## Configuration Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     Application Entry Point                  │
│                    (e.g., `para start`)                     │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Config::load_or_create()                   │
│         (in src/config/mod.rs, line 122-124)               │
│                           │                                  │
│                           ▼                                  │
│         ConfigManager::load_with_project_config()           │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│           ConfigManager::load_with_project_config()          │
│          (in src/config/manager.rs, line 48-57)            │
│                                                             │
│  1. Load user config:                                       │
│     let user_config = Self::load_or_create()?;             │
│     → Loads from ~/Library/Application Support/para/config.json
│                                                             │
│  2. Load project config if available:                       │
│     let project_config = Self::load_project_config()?;      │
│     → Searches up from CWD for .para/config.json           │
│                                                             │
│  3. Merge and return:                                       │
│     Ok(Self::merge_configs(user_config, project_config))    │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    merge_configs()                           │
│          (in src/config/manager.rs, line 170-206)          │
│                                                             │
│  Merging Rules:                                            │
│                                                             │
│  1. Sandbox Settings:                                       │
│     - enabled: project OVERRIDES user                      │
│     - profile: project OVERRIDES user                      │
│     - allowed_domains: MERGE arrays (deduplicated)         │
│                                                             │
│  2. IDE Settings:                                           │
│     - ide.preferred → ide.name: project OVERRIDES user     │
│                                                             │
│  3. All other settings: user config values remain          │
└─────────────────────────────────────────────────────────────┘
```

## Example Merge

### User Config (Global)
```json
{
  "ide": {
    "name": "claude",
    "command": "claude"
  },
  "sandbox": {
    "enabled": false,
    "profile": "permissive-open",
    "allowed_domains": ["github.com", "gitlab.com"]
  },
  "git": {
    "auto_stage": true
  }
}
```

### Project Config (Local)
```json
{
  "sandbox": {
    "enabled": true,
    "profile": "standard",
    "allowed_domains": ["api.example.com", "npmjs.org"]
  },
  "ide": {
    "preferred": "cursor"
  }
}
```

### Result (Merged)
```json
{
  "ide": {
    "name": "cursor",        // ← Overridden by project.ide.preferred
    "command": "claude"      // ← Unchanged from user config
  },
  "sandbox": {
    "enabled": true,         // ← Overridden by project
    "profile": "standard",   // ← Overridden by project
    "allowed_domains": [     // ← Merged and deduplicated
      "api.example.com",
      "github.com", 
      "gitlab.com",
      "npmjs.org"
    ]
  },
  "git": {
    "auto_stage": true       // ← Unchanged from user config
  }
}
```

## Key Implementation Details

1. **Both configs are loaded**: The system doesn't replace one with the other, it loads both and intelligently merges them.

2. **Project config is partial**: Project config only needs to specify what it wants to override or add to.

3. **Smart merging**: 
   - Simple values (booleans, strings) are overridden
   - Arrays are merged and deduplicated
   - Missing fields in project config don't affect user config

4. **Search behavior**: Project config is found by walking up from the current directory until finding `.para/config.json` or reaching the filesystem root.

5. **Validation**: The final merged config is validated before use.

## When Merging Happens

The merge happens automatically whenever any Para command loads configuration:
- `para start`
- `para finish`
- `para resume`
- etc.

Commands that don't need config (like `para config` itself) load configs separately without merging.