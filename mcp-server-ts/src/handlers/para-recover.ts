import { ParaRecoverArgs, ToolHandler } from './types.js';

export const handleParaRecover: ToolHandler = async (args: ParaRecoverArgs, runParaCommand) => {
  const cmdArgs = ["recover"];
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  return await runParaCommand(cmdArgs);
};