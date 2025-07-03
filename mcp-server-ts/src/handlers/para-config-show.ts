import { ToolHandler } from './types.js';

export const handleParaConfigShow: ToolHandler = async (_args, runParaCommand) => {
  return await runParaCommand(["config", "show"]);
};