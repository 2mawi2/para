/**
 * Management Tools - Handle para configuration and cleanup operations
 */

import { ParaBinaryInterface, ExecResult } from "../para/binary-interface.js";

export class ManagementTools {
  constructor(private binaryInterface: ParaBinaryInterface) {}

  /**
   * Recover a para session
   */
  async recover(args: { session_name?: string }): Promise<ExecResult> {
    const command = "recover";
    const cmdArgs: string[] = [];
    
    if (args.session_name) {
      cmdArgs.push(args.session_name);
    }
    
    return await this.binaryInterface.executeCommand(command, cmdArgs);
  }

  /**
   * Cancel (delete) a para session
   */
  async cancel(args: { session_name?: string }): Promise<ExecResult> {
    const command = "cancel";
    const cmdArgs: string[] = [];
    
    if (args.session_name) {
      cmdArgs.push(args.session_name);
    }
    
    return await this.binaryInterface.executeCommand(command, cmdArgs);
  }

  /**
   * Show para configuration
   */
  async configShow(): Promise<ExecResult> {
    const command = "config";
    const cmdArgs: string[] = ["show"];
    
    return await this.binaryInterface.executeCommand(command, cmdArgs);
  }
}