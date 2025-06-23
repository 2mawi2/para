import { CommandHandler } from "./base-handler.js";
import { ParaStartHandler } from "./handlers/para-start-handler.js";
import { ParaFinishHandler } from "./handlers/para-finish-handler.js";
import { ParaDispatchHandler } from "./handlers/para-dispatch-handler.js";
import { ParaListHandler } from "./handlers/para-list-handler.js";
import { ParaRecoverHandler } from "./handlers/para-recover-handler.js";
import { ParaResumeHandler } from "./handlers/para-resume-handler.js";
import { ParaCancelHandler } from "./handlers/para-cancel-handler.js";
import { ParaStatusHandler } from "./handlers/para-status-handler.js";
import { ParaConfigHandler } from "./handlers/para-config-handler.js";

export class CommandRegistry {
  private handlers: Map<string, CommandHandler> = new Map();

  constructor(paraBinary: string) {
    this.registerHandlers(paraBinary);
  }

  private registerHandlers(paraBinary: string): void {
    const handlers = [
      new ParaStartHandler(paraBinary),
      new ParaFinishHandler(paraBinary),
      new ParaDispatchHandler(paraBinary),
      new ParaListHandler(paraBinary),
      new ParaRecoverHandler(paraBinary),
      new ParaResumeHandler(paraBinary),
      new ParaCancelHandler(paraBinary),
      new ParaStatusHandler(paraBinary),
      new ParaConfigHandler(paraBinary),
    ];

    for (const handler of handlers) {
      const toolDef = handler.getToolDefinition();
      this.handlers.set(toolDef.name, handler);
    }
  }

  getHandler(toolName: string): CommandHandler | undefined {
    return this.handlers.get(toolName);
  }

  getAllHandlers(): CommandHandler[] {
    return Array.from(this.handlers.values());
  }

  getToolNames(): string[] {
    return Array.from(this.handlers.keys());
  }
}