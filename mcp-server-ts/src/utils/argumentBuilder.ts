#!/usr/bin/env node
/**
 * Para Command Argument Building Utilities
 * Handles building command arguments for different para commands from MCP tool arguments
 */

/**
 * Builds arguments for the para start command
 */
export function buildParaStartArgs(args: any): string[] {
  const cmdArgs = ["start"];
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  if (args.dangerously_skip_permissions) {
    cmdArgs.push("--dangerously-skip-permissions");
  }
  return cmdArgs;
}

/**
 * Builds arguments for the para finish command
 */
export function buildParaFinishArgs(args: any): string[] {
  const cmdArgs = ["finish"];
  cmdArgs.push(args.commit_message);
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.branch) {
    cmdArgs.push("--branch", args.branch);
  }
  return cmdArgs;
}

/**
 * Builds arguments for the para dispatch command
 */
export function buildParaDispatchArgs(args: any): string[] {
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

  return cmdArgs;
}

/**
 * Builds arguments for the para list command
 */
export function buildParaListArgs(args: any): string[] {
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
  return cmdArgs;
}

/**
 * Builds arguments for the para recover command
 */
export function buildParaRecoverArgs(args: any): string[] {
  const cmdArgs = ["recover"];
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  return cmdArgs;
}

/**
 * Builds arguments for the para resume command
 */
export function buildParaResumeArgs(args: any): string[] {
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
  return cmdArgs;
}

/**
 * Builds arguments for the para cancel command
 */
export function buildParaCancelArgs(args: any): string[] {
  const cmdArgs = ["cancel"];
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  if (args.force) {
    cmdArgs.push("--force");
  }
  return cmdArgs;
}

/**
 * Builds arguments for the para status command
 */
export function buildParaStatusArgs(args: any): string[] {
  const cmdArgs = ["status", "show"];
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.json) {
    cmdArgs.push("--json");
  }
  return cmdArgs;
}