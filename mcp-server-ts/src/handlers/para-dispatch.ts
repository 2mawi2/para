import { ParaDispatchArgs, ToolHandler } from './types.js';

export const handleParaDispatch: ToolHandler = async (args: ParaDispatchArgs, runParaCommand) => {
  const cmdArgs = ["dispatch"];
  cmdArgs.push(args.session_name);

  if (args.file) {
    cmdArgs.push("--file", args.file);
  } else if (args.task_description) {
    cmdArgs.push(args.task_description);
  }

  if (args.dangerously_skip_permissions) {
    cmdArgs.push("--dangerously-skip-permissions");
  }

  return await runParaCommand(cmdArgs);
};