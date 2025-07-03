import { ParaResumeArgs, ToolHandler } from './types.js';

export const handleParaResume: ToolHandler = async (args: ParaResumeArgs, runParaCommand) => {
  const cmdArgs = ["resume"];
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.prompt) {
    cmdArgs.push("--prompt", args.prompt);
  }
  if (args.file) {
    cmdArgs.push("--file", args.file);
  }
  return await runParaCommand(cmdArgs);
};