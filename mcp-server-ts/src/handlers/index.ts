import { handleParaStart } from './para-start.js';
import { handleParaFinish } from './para-finish.js';
import { handleParaDispatch } from './para-dispatch.js';
import { handleParaList } from './para-list.js';
import { handleParaRecover } from './para-recover.js';
import { handleParaResume } from './para-resume.js';
import { handleParaConfigShow } from './para-config-show.js';
import { handleParaCancel } from './para-cancel.js';
import { handleParaStatusShow } from './para-status-show.js';
import { ToolHandler } from './types.js';

export const toolHandlers: Record<string, ToolHandler> = {
  para_start: handleParaStart,
  para_finish: handleParaFinish,
  para_dispatch: handleParaDispatch,
  para_list: handleParaList,
  para_recover: handleParaRecover,
  para_resume: handleParaResume,
  para_config_show: handleParaConfigShow,
  para_cancel: handleParaCancel,
  para_status_show: handleParaStatusShow,
};

export * from './types.js';