/**
 * Para config set tool implementation
 */

import { ToolDefinition } from '../types/mcp-types.js';
import { ParaConfigSetArgs } from '../types/para-args.js';
import { ParaExecutor } from '../execution/para-executor.js';

export const configSetToolDefinition: ToolDefinition = {
  name: "para_config_set",
  description: "Set para configuration values using JSON path notation. Supports setting IDE, Git, directories, and session configuration. Values are automatically typed (string, boolean, number).",
  inputSchema: {
    type: "object",
    properties: {
      path: {
        type: "string",
        description: "JSON path using dot notation (e.g., 'ide.name', 'git.auto_stage', 'ide.wrapper.command', 'session.auto_cleanup_days')"
      },
      value: {
        oneOf: [
          { type: "string" },
          { type: "boolean" },
          { type: "number" }
        ],
        description: "Value to set - automatically typed based on input"
      }
    },
    required: ["path", "value"]
  }
};

export async function executeConfigSetTool(args: ParaConfigSetArgs, executor: ParaExecutor): Promise<string> {
  const cmdArgs = ["config", "set", args.path, String(args.value)];
  return executor.runCommand(cmdArgs);
}