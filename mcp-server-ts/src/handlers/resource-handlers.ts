export type ResourceHandler = (_runParaCommand: (_cmdArgs: string[]) => Promise<string>) => Promise<string>;

export const resourceHandlers: Record<string, ResourceHandler> = {
  "para://current-session": async (runParaCommand) => {
    return await runParaCommand(["list", "--current"]);
  },
  "para://config": async (runParaCommand) => {
    return await runParaCommand(["config", "show"]);
  },
};