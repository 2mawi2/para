/**
 * Session Tools - Handle para session lifecycle operations
 */

import { ParaBinaryInterface, ExecResult } from "../para/binary-interface.js";

export class SessionTools {
  constructor(private binaryInterface: ParaBinaryInterface) {}

  /**
   * Start a new para session
   */
  async start(args: { session_name?: string; dangerously_skip_permissions?: boolean }): Promise<ExecResult> {
    const command = "start";
    const cmdArgs: string[] = [];
    
    if (args.session_name) {
      cmdArgs.push(args.session_name);
    }
    
    if (args.dangerously_skip_permissions) {
      cmdArgs.push("--dangerously-skip-permissions");
    }
    
    return await this.binaryInterface.executeCommand(command, cmdArgs);
  }

  /**
   * Finish a para session
   */
  async finish(args: { commit_message: string; session?: string; branch?: string }): Promise<ExecResult> {
    const command = "finish";
    const cmdArgs: string[] = [args.commit_message];
    
    if (args.session) {
      cmdArgs.push(args.session);
    }
    
    if (args.branch) {
      cmdArgs.push("--branch", args.branch);
    }
    
    return await this.binaryInterface.executeCommand(command, cmdArgs);
  }

  /**
   * Dispatch an agent to work on a task
   */
  async dispatch(args: { 
    session_name: string; 
    task_description?: string; 
    file?: string; 
    dangerously_skip_permissions?: boolean 
  }): Promise<ExecResult> {
    const command = "dispatch";
    const cmdArgs: string[] = [args.session_name];
    
    if (args.file) {
      cmdArgs.push("--file", args.file);
    } else if (args.task_description) {
      cmdArgs.push(args.task_description);
    }
    
    if (args.dangerously_skip_permissions) {
      cmdArgs.push("--dangerously-skip-permissions");
    }
    
    return await this.binaryInterface.executeCommand(command, cmdArgs);
  }
}