export interface ParaStartArgs {
  name_or_session?: string;
  prompt?: string;
  file?: string;
  dangerously_skip_permissions?: boolean;
  container?: boolean;
  docker_args?: string[];
  sandbox?: boolean;
  no_sandbox?: boolean;
  sandbox_profile?: string;
  docker_image?: string;
  allow_domains?: string;
  no_forward_keys?: boolean;
  setup_script?: string;
}

export interface ParaFinishArgs {
  commit_message: string;
  session?: string;
  branch?: string;
}

export interface ParaListArgs {
  verbose?: boolean;
  archived?: boolean;
  quiet?: boolean;
}

export interface ParaResumeArgs {
  session?: string;
  prompt?: string;
  file?: string;
  dangerously_skip_permissions?: boolean;
  sandbox?: boolean;
  no_sandbox?: boolean;
  sandbox_profile?: string;
}

export interface ParaRecoverArgs {
  session_name?: string;
}

export interface ParaCancelArgs {
  session_name?: string;
  force?: boolean;
}

export interface ParaStatusShowArgs {
  session?: string;
  json?: boolean;
}

export interface ParaConfigSetArgs {
  path: string;
  value: string | boolean | number;
}