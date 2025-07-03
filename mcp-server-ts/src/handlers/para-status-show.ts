import { ParaStatusShowArgs, ToolHandler } from './types.js';

export const handleParaStatusShow: ToolHandler = async (args: ParaStatusShowArgs, runParaCommand) => {
  const cmdArgs = ["status", "show"];
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.json) {
    cmdArgs.push("--json");
  }
  return await runParaCommand(cmdArgs);
};