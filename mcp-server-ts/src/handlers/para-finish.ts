import { ParaFinishArgs, ToolHandler } from './types.js';

export const handleParaFinish: ToolHandler = async (args: ParaFinishArgs, runParaCommand) => {
  const cmdArgs = ["finish"];
  cmdArgs.push(args.commit_message);
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.branch) {
    cmdArgs.push("--branch", args.branch);
  }
  return await runParaCommand(cmdArgs);
};