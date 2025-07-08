import { ParaCancelArgs } from '../types';

export class CancelCommandBuilder {
  static build(args: ParaCancelArgs): string[] {
    const cmdArgs = ["cancel"];
    if (args.session_name) {
      cmdArgs.push(args.session_name);
    }
    if (args.force) {
      cmdArgs.push("--force");
    }
    return cmdArgs;
  }
}