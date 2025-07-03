import { ParaCancelArgs, ToolHandler } from './types.js';

export const handleParaCancel: ToolHandler = async (args: ParaCancelArgs, runParaCommand) => {
  const cmdArgs = ["cancel"];
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  if (args.force) {
    cmdArgs.push("--force");
  }
  return await runParaCommand(cmdArgs);
};