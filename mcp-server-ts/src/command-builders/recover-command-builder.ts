import { ParaRecoverArgs } from '../types';

export class RecoverCommandBuilder {
  static build(args: ParaRecoverArgs): string[] {
    const cmdArgs = ["recover"];
    if (args.session_name) {
      cmdArgs.push(args.session_name);
    }
    return cmdArgs;
  }
}