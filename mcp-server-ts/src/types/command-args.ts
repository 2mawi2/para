/**
 * Type definitions for Para MCP Server command arguments
 */

export interface ParaStartArgs {
  session_name?: string;
  dangerously_skip_permissions?: boolean;
}

export interface ParaFinishArgs {
  commit_message: string;
  session?: string;
  branch?: string;
}

export interface ParaDispatchArgs {
  session_name: string;
  task_description?: string;
  file?: string;
  dangerously_skip_permissions?: boolean;
}

export interface ParaListArgs {
  verbose?: boolean;
  archived?: boolean;
  quiet?: boolean;
}

export interface ParaRecoverArgs {
  session_name?: string;
}

export interface ParaResumeArgs {
  session?: string;
  prompt?: string;
  file?: string;
}

export interface ParaCancelArgs {
  session_name?: string;
  force?: boolean;
}

export interface ParaStatusShowArgs {
  session?: string;
  json?: boolean;
}

export interface ParaConfigShowArgs {
  // No arguments for config show
}