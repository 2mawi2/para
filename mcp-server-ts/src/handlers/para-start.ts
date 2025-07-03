import { ParaStartArgs, ToolHandler } from './types.js';

export const handleParaStart: ToolHandler = async (args: ParaStartArgs, runParaCommand) => {
  const cmdArgs = ["start"];
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  if (args.dangerously_skip_permissions) {
    cmdArgs.push("--dangerously-skip-permissions");
  }
  return await runParaCommand(cmdArgs);
};