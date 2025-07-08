import {
  StartCommandBuilder,
  FinishCommandBuilder,
  ResumeCommandBuilder,
  ListCommandBuilder,
  RecoverCommandBuilder,
  CancelCommandBuilder,
  StatusCommandBuilder,
  ConfigCommandBuilder
} from './command-builders/index';

describe('Command Argument Building', () => {
  test('should build para_start command with session name', () => {
    const args = { name_or_session: 'test-session' };
    const result = StartCommandBuilder.build(args);
    expect(result).toEqual(['start', 'test-session']);
  });

  test('should build para_start command with dangerously_skip_permissions', () => {
    const args = { name_or_session: 'test-session', dangerously_skip_permissions: true };
    const result = StartCommandBuilder.build(args);
    expect(result).toEqual(['start', 'test-session', '--dangerously-skip-permissions']);
  });

  test('should build unified para_start command with prompt', () => {
    const args = { prompt: 'implement feature X' };
    const result = StartCommandBuilder.build(args);
    expect(result).toEqual(['start', 'implement feature X']);
  });

  test('should build unified para_start command with name and prompt', () => {
    const args = { name_or_session: 'my-session', prompt: 'add feature Y' };
    const result = StartCommandBuilder.build(args);
    expect(result).toEqual(['start', 'my-session', 'add feature Y']);
  });

  test('should build unified para_start command with file', () => {
    const args = { file: 'tasks/auth.md' };
    const result = StartCommandBuilder.build(args);
    expect(result).toEqual(['start', '--file', 'tasks/auth.md']);
  });

  test('should build para_finish command with required commit message', () => {
    const args = { commit_message: 'Add new feature' };
    const result = FinishCommandBuilder.build(args);
    expect(result).toEqual(['finish', 'Add new feature']);
  });

  test('should build para_finish command with session and branch', () => {
    const args = { commit_message: 'Add new feature', session: 'test-session', branch: 'feature-branch' };
    const result = FinishCommandBuilder.build(args);
    expect(result).toEqual(['finish', 'Add new feature', 'test-session', '--branch', 'feature-branch']);
  });

  test('should build para_resume command with session only', () => {
    const args = { session: 'my-session' };
    const result = ResumeCommandBuilder.build(args);
    expect(result).toEqual(['resume', 'my-session']);
  });

  test('should build para_resume command with session and prompt', () => {
    const args = { session: 'my-session', prompt: 'add error handling' };
    const result = ResumeCommandBuilder.build(args);
    expect(result).toEqual(['resume', 'my-session', '--prompt', 'add error handling']);
  });

  test('should build para_resume command with all options', () => {
    const args = { 
      session: 'my-session', 
      file: 'context.md', 
      dangerously_skip_permissions: true, 
      sandbox: true 
    };
    const result = ResumeCommandBuilder.build(args);
    expect(result).toEqual(['resume', 'my-session', '--file', 'context.md', '--dangerously-skip-permissions', '--sandbox']);
  });

  test('should build para_list command with all flags', () => {
    const args = { verbose: true, archived: true, quiet: true };
    const result = ListCommandBuilder.build(args);
    expect(result).toEqual(['list', '--verbose', '--archived', '--quiet']);
  });

  test('should build para_recover command with session name', () => {
    const args = { session_name: 'lost-session' };
    const result = RecoverCommandBuilder.build(args);
    expect(result).toEqual(['recover', 'lost-session']);
  });

  test('should build para_cancel command with force flag', () => {
    const args = { session_name: 'abandoned-session', force: true };
    const result = CancelCommandBuilder.build(args);
    expect(result).toEqual(['cancel', 'abandoned-session', '--force']);
  });

  test('should build para_status_show command with JSON output', () => {
    const args = { session: 'agent-session', json: true };
    const result = StatusCommandBuilder.build(args);
    expect(result).toEqual(['status', 'show', 'agent-session', '--json']);
  });

  test('should build para_config_set command with string value', () => {
    const args = { path: 'ide.name', value: 'cursor' };
    const result = ConfigCommandBuilder.buildSet(args);
    expect(result).toEqual(['config', 'set', 'ide.name', 'cursor']);
  });

  test('should build para_config_set command with boolean value', () => {
    const args = { path: 'git.auto_stage', value: true };
    const result = ConfigCommandBuilder.buildSet(args);
    expect(result).toEqual(['config', 'set', 'git.auto_stage', 'true']);
  });

  test('should build para_config_set command with number value', () => {
    const args = { path: 'session.auto_cleanup_days', value: 14 };
    const result = ConfigCommandBuilder.buildSet(args);
    expect(result).toEqual(['config', 'set', 'session.auto_cleanup_days', '14']);
  });

  test('should build para_config_set command with nested path', () => {
    const args = { path: 'ide.wrapper.command', value: 'cursor' };
    const result = ConfigCommandBuilder.buildSet(args);
    expect(result).toEqual(['config', 'set', 'ide.wrapper.command', 'cursor']);
  });
});