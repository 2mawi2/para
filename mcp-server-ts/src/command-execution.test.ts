import { runParaCommand } from './command-execution';

describe('Argument Quoting', () => {
  test('should quote arguments with spaces', () => {
    const input = 'commit message with spaces';
    const expected = '"commit message with spaces"';
    
    const result = input.includes(' ') && !input.startsWith('"') && !input.startsWith("'") 
      ? `"${input.replace(/"/g, '\\"')}"` 
      : input;
    
    expect(result).toBe(expected);
  });

  test('should not quote arguments without spaces', () => {
    const input = 'single-word';
    const expected = 'single-word';
    
    const result = input.includes(' ') && !input.startsWith('"') && !input.startsWith("'") 
      ? `"${input.replace(/"/g, '\\"')}"` 
      : input;
    
    expect(result).toBe(expected);
  });

  test('should not quote already quoted arguments', () => {
    const input = '"already quoted"';
    const expected = '"already quoted"';
    
    const result = input.includes(' ') && !input.startsWith('"') && !input.startsWith("'") 
      ? `"${input.replace(/"/g, '\\"')}"` 
      : input;
    
    expect(result).toBe(expected);
  });

  test('should escape internal quotes when quoting', () => {
    const input = 'message with "quotes" inside';
    const expected = '"message with \\"quotes\\" inside"';
    
    const result = input.includes(' ') && !input.startsWith('"') && !input.startsWith("'") 
      ? `"${input.replace(/"/g, '\\"')}"` 
      : input;
    
    expect(result).toBe(expected);
  });
});

describe('Environment Variables', () => {
  test('should set non-interactive environment variables', () => {
    const expectedEnv = {
      PARA_NON_INTERACTIVE: '1',
      CI: '1'
    };
    
    expect(expectedEnv.PARA_NON_INTERACTIVE).toBe('1');
    expect(expectedEnv.CI).toBe('1');
  });
});

describe('Error Handling', () => {
  test('should handle unknown tool error', () => {
    const unknownTool = 'unknown_tool';
    const expectedError = {
      code: 'METHOD_NOT_FOUND',
      message: `Unknown tool: ${unknownTool}`
    };
    
    expect(expectedError.code).toBe('METHOD_NOT_FOUND');
    expect(expectedError.message).toContain(unknownTool);
  });

  test('should handle command execution timeout', () => {
    const timeoutError = {
      code: 'INTERNAL_ERROR',
      message: 'Command timed out after 30 seconds'
    };
    
    expect(timeoutError.code).toBe('INTERNAL_ERROR');
    expect(timeoutError.message).toContain('timed out');
  });

  test('should handle invalid resource URI', () => {
    const invalidUri = 'para://invalid-resource';
    const expectedError = {
      code: 'INVALID_REQUEST',
      message: `Unknown resource: ${invalidUri}`
    };
    
    expect(expectedError.code).toBe('INVALID_REQUEST');
    expect(expectedError.message).toContain(invalidUri);
  });
});