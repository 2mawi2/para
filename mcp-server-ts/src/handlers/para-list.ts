import { ParaListArgs, ToolHandler } from './types.js';

export const handleParaList: ToolHandler = async (args: ParaListArgs, runParaCommand) => {
  const cmdArgs = ["list"];
  if (args.verbose) {
    cmdArgs.push("--verbose");
  }
  if (args.archived) {
    cmdArgs.push("--archived");
  }
  if (args.quiet) {
    cmdArgs.push("--quiet");
  }
  return await runParaCommand(cmdArgs);
};