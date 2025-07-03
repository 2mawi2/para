/**
 * Index file for all Para MCP command handlers
 */

// Command handlers
export { handleParaStart, paraStartTool } from "./start.js";
export { handleParaFinish, paraFinishTool } from "./finish.js";
export { handleParaDispatch, paraDispatchTool } from "./dispatch.js";
export { handleParaList, paraListTool } from "./list.js";
export { handleParaRecover, paraRecoverTool } from "./recover.js";
export { handleParaResume, paraResumeTool } from "./resume.js";
export { handleParaCancel, paraCancelTool } from "./cancel.js";
export { handleParaConfigShow, paraConfigShowTool } from "./config-show.js";
export { handleParaStatusShow, paraStatusShowTool } from "./status-show.js";

// Import all tools and handlers for internal use
import { paraStartTool, handleParaStart } from "./start.js";
import { paraFinishTool, handleParaFinish } from "./finish.js";
import { paraDispatchTool, handleParaDispatch } from "./dispatch.js";
import { paraListTool, handleParaList } from "./list.js";
import { paraRecoverTool, handleParaRecover } from "./recover.js";
import { paraResumeTool, handleParaResume } from "./resume.js";
import { paraCancelTool, handleParaCancel } from "./cancel.js";
import { paraConfigShowTool, handleParaConfigShow } from "./config-show.js";
import { paraStatusShowTool, handleParaStatusShow } from "./status-show.js";

// Tool definitions array for easy registration
export const allParaTools = [
  paraStartTool,
  paraFinishTool,
  paraDispatchTool,
  paraListTool,
  paraRecoverTool,
  paraResumeTool,
  paraConfigShowTool,
  paraCancelTool,
  paraStatusShowTool,
] as const;

// Command name to handler mapping for easy lookup
export const commandHandlers = {
  para_start: handleParaStart,
  para_finish: handleParaFinish,
  para_dispatch: handleParaDispatch,
  para_list: handleParaList,
  para_recover: handleParaRecover,
  para_resume: handleParaResume,
  para_config_show: handleParaConfigShow,
  para_cancel: handleParaCancel,
  para_status_show: handleParaStatusShow,
} as const;