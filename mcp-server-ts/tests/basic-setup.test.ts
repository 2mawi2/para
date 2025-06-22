/**
 * Basic setup tests to verify Jest and TypeScript are working correctly
 */

describe('Basic Setup', () => {
  test('should run basic test', () => {
    expect(1 + 1).toBe(2);
  });

  test('should handle async tests', async () => {
    const result = await Promise.resolve('hello');
    expect(result).toBe('hello');
  });

  test('should be able to mock functions', () => {
    const mockFn = jest.fn();
    mockFn('test');
    expect(mockFn).toHaveBeenCalledWith('test');
  });
});