#!/usr/bin/env node
/**
 * Para Service
 * Core business logic for executing para commands
 */

import { findParaBinary } from "../utils/binaryDiscovery.js";
import { runParaCommand } from "../utils/commandExecution.js";
import {
  buildParaStartArgs,
  buildParaFinishArgs,
  buildParaDispatchArgs,
  buildParaListArgs,
  buildParaRecoverArgs,
  buildParaResumeArgs,
  buildParaCancelArgs,
  buildParaStatusArgs
} from "../utils/argumentBuilder.js";

/**
 * Para Service class that handles all para command execution
 */
export class ParaService {
  private paraBinary: string;

  constructor() {
    this.paraBinary = findParaBinary();
    console.error(`Para MCP server using para binary: ${this.paraBinary}`);
  }

  /**
   * Executes para start command
   */
  async executeStart(args: any): Promise<string> {
    const cmdArgs = buildParaStartArgs(args);
    return await runParaCommand(cmdArgs, this.paraBinary);
  }

  /**
   * Executes para finish command
   */
  async executeFinish(args: any): Promise<string> {
    const cmdArgs = buildParaFinishArgs(args);
    return await runParaCommand(cmdArgs, this.paraBinary);
  }

  /**
   * Executes para dispatch command
   */
  async executeDispatch(args: any): Promise<string> {
    const cmdArgs = buildParaDispatchArgs(args);
    return await runParaCommand(cmdArgs, this.paraBinary);
  }

  /**
   * Executes para list command
   */
  async executeList(args: any): Promise<string> {
    const cmdArgs = buildParaListArgs(args);
    return await runParaCommand(cmdArgs, this.paraBinary);
  }

  /**
   * Executes para recover command
   */
  async executeRecover(args: any): Promise<string> {
    const cmdArgs = buildParaRecoverArgs(args);
    return await runParaCommand(cmdArgs, this.paraBinary);
  }

  /**
   * Executes para resume command
   */
  async executeResume(args: any): Promise<string> {
    const cmdArgs = buildParaResumeArgs(args);
    return await runParaCommand(cmdArgs, this.paraBinary);
  }

  /**
   * Executes para config show command
   */
  async executeConfigShow(): Promise<string> {
    return await runParaCommand(["config", "show"], this.paraBinary);
  }

  /**
   * Executes para cancel command
   */
  async executeCancel(args: any): Promise<string> {
    const cmdArgs = buildParaCancelArgs(args);
    return await runParaCommand(cmdArgs, this.paraBinary);
  }

  /**
   * Executes para status show command
   */
  async executeStatusShow(args: any): Promise<string> {
    const cmdArgs = buildParaStatusArgs(args);
    return await runParaCommand(cmdArgs, this.paraBinary);
  }

  /**
   * Gets current session information
   */
  async getCurrentSession(): Promise<string> {
    return await runParaCommand(["list", "--current"], this.paraBinary);
  }
}