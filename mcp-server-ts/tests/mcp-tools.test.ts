/**
 * Tests for MCP tool argument building logic
 * These test the argument construction functions extracted from the monolithic file
 */

const {
  buildParaStartArgs,
  buildParaFinishArgs,
  buildParaDispatchArgs,
  buildParaListArgs,
  buildParaRecoverArgs,
  buildParaResumeArgs,
  buildParaCancelArgs,
  buildParaStatusArgs
} = require('../src/test-utils');

describe('MCP Tool Argument Building', () => {
  describe('buildParaStartArgs', () => {
    test('should build basic start command', () => {
      const args = {};
      const result = buildParaStartArgs(args);
      expect(result).toEqual(['start']);
    });

    test('should include session name when provided', () => {
      const args = { session_name: 'test-session' };
      const result = buildParaStartArgs(args);
      expect(result).toEqual(['start', 'test-session']);
    });

    test('should include dangerously-skip-permissions flag', () => {
      const args = { 
        session_name: 'test-session',
        dangerously_skip_permissions: true 
      };
      const result = buildParaStartArgs(args);
      expect(result).toEqual(['start', 'test-session', '--dangerously-skip-permissions']);
    });
  });

  describe('buildParaFinishArgs', () => {
    test('should build basic finish command with commit message', () => {
      const args = { commit_message: 'Add feature' };
      const result = buildParaFinishArgs(args);
      expect(result).toEqual(['finish', 'Add feature']);
    });

    test('should include session when provided', () => {
      const args = { 
        commit_message: 'Add feature',
        session: 'test-session'
      };
      const result = buildParaFinishArgs(args);
      expect(result).toEqual(['finish', 'Add feature', 'test-session']);
    });

    test('should include custom branch when provided', () => {
      const args = { 
        commit_message: 'Add feature',
        branch: 'feature/auth'
      };
      const result = buildParaFinishArgs(args);
      expect(result).toEqual(['finish', 'Add feature', '--branch', 'feature/auth']);
    });
  });

  describe('buildParaDispatchArgs', () => {
    test('should build basic dispatch command with session name', () => {
      const args = { session_name: 'agent-session' };
      const result = buildParaDispatchArgs(args);
      expect(result).toEqual(['dispatch', 'agent-session']);
    });

    test('should include file parameter when provided', () => {
      const args = { 
        session_name: 'agent-session',
        file: 'tasks/task.md'
      };
      const result = buildParaDispatchArgs(args);
      expect(result).toEqual(['dispatch', 'agent-session', '--file', 'tasks/task.md']);
    });

    test('should include task description when provided', () => {
      const args = { 
        session_name: 'agent-session',
        task_description: 'Implement user auth'
      };
      const result = buildParaDispatchArgs(args);
      expect(result).toEqual(['dispatch', 'agent-session', 'Implement user auth']);
    });

    test('should prefer file over task description', () => {
      const args = { 
        session_name: 'agent-session',
        file: 'tasks/task.md',
        task_description: 'This should be ignored'
      };
      const result = buildParaDispatchArgs(args);
      expect(result).toEqual(['dispatch', 'agent-session', '--file', 'tasks/task.md']);
    });

    test('should include dangerously-skip-permissions flag', () => {
      const args = { 
        session_name: 'agent-session',
        task_description: 'Implement feature',
        dangerously_skip_permissions: true
      };
      const result = buildParaDispatchArgs(args);
      expect(result).toEqual(['dispatch', 'agent-session', 'Implement feature', '--dangerously-skip-permissions']);
    });
  });

  describe('buildParaListArgs', () => {
    test('should build basic list command', () => {
      const args = {};
      const result = buildParaListArgs(args);
      expect(result).toEqual(['list']);
    });

    test('should include verbose flag', () => {
      const args = { verbose: true };
      const result = buildParaListArgs(args);
      expect(result).toEqual(['list', '--verbose']);
    });

    test('should include archived flag', () => {
      const args = { archived: true };
      const result = buildParaListArgs(args);
      expect(result).toEqual(['list', '--archived']);
    });

    test('should include quiet flag', () => {
      const args = { quiet: true };
      const result = buildParaListArgs(args);
      expect(result).toEqual(['list', '--quiet']);
    });

    test('should include multiple flags', () => {
      const args = { verbose: true, archived: true };
      const result = buildParaListArgs(args);
      expect(result).toEqual(['list', '--verbose', '--archived']);
    });
  });

  describe('buildParaStatusArgs', () => {
    test('should build basic status command', () => {
      const args = {};
      const result = buildParaStatusArgs(args);
      expect(result).toEqual(['status', 'show']);
    });

    test('should include session when provided', () => {
      const args = { session: 'agent-session' };
      const result = buildParaStatusArgs(args);
      expect(result).toEqual(['status', 'show', 'agent-session']);
    });

    test('should include json flag', () => {
      const args = { json: true };
      const result = buildParaStatusArgs(args);
      expect(result).toEqual(['status', 'show', '--json']);
    });

    test('should include both session and json flag', () => {
      const args = { session: 'agent-session', json: true };
      const result = buildParaStatusArgs(args);
      expect(result).toEqual(['status', 'show', 'agent-session', '--json']);
    });
  });

  describe('buildParaRecoverArgs', () => {
    test('should build basic recover command', () => {
      const args = {};
      const result = buildParaRecoverArgs(args);
      expect(result).toEqual(['recover']);
    });

    test('should include session name when provided', () => {
      const args = { session_name: 'old-session' };
      const result = buildParaRecoverArgs(args);
      expect(result).toEqual(['recover', 'old-session']);
    });
  });

  describe('buildParaResumeArgs', () => {
    test('should build basic resume command', () => {
      const args = {};
      const result = buildParaResumeArgs(args);
      expect(result).toEqual(['resume']);
    });

    test('should include session when provided', () => {
      const args = { session: 'existing-session' };
      const result = buildParaResumeArgs(args);
      expect(result).toEqual(['resume', 'existing-session']);
    });

    test('should include prompt when provided', () => {
      const args = { prompt: 'Continue with feature X' };
      const result = buildParaResumeArgs(args);
      expect(result).toEqual(['resume', '--prompt', 'Continue with feature X']);
    });

    test('should include file when provided', () => {
      const args = { file: 'tasks/continuation.md' };
      const result = buildParaResumeArgs(args);
      expect(result).toEqual(['resume', '--file', 'tasks/continuation.md']);
    });
  });

  describe('buildParaCancelArgs', () => {
    test('should build basic cancel command', () => {
      const args = {};
      const result = buildParaCancelArgs(args);
      expect(result).toEqual(['cancel']);
    });

    test('should include session name when provided', () => {
      const args = { session_name: 'session-to-cancel' };
      const result = buildParaCancelArgs(args);
      expect(result).toEqual(['cancel', 'session-to-cancel']);
    });

    test('should include force flag', () => {
      const args = { force: true };
      const result = buildParaCancelArgs(args);
      expect(result).toEqual(['cancel', '--force']);
    });

    test('should include both session and force flag', () => {
      const args = { session_name: 'session-to-cancel', force: true };
      const result = buildParaCancelArgs(args);
      expect(result).toEqual(['cancel', 'session-to-cancel', '--force']);
    });
  });
});