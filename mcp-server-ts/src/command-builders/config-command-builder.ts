import { ParaConfigSetArgs } from '../types';

export class ConfigCommandBuilder {
  static buildShow(): string[] {
    return ["config", "show"];
  }

  static buildSet(args: ParaConfigSetArgs): string[] {
    return ["config", "set", args.path, String(args.value)];
  }
}