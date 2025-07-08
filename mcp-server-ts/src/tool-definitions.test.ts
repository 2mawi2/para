import { PARA_TOOLS } from './tool-definitions';

describe('Tool Definitions', () => {
  test('should define para_start tool with correct schema', () => {
    const tool = PARA_TOOLS.find(t => t.name === 'para_start');
    expect(tool).toBeTruthy();
    expect(tool?.name).toBe("para_start");
    expect(tool?.description).toContain("Start NEW para sessions");
    expect(tool?.inputSchema.properties).toHaveProperty('name_or_session');
    expect(tool?.inputSchema.properties).toHaveProperty('dangerously_skip_permissions');
  });

  test('should define para_resume tool with correct schema', () => {
    const tool = PARA_TOOLS.find(t => t.name === 'para_resume');
    expect(tool).toBeTruthy();
    expect(tool?.name).toBe("para_resume");
    expect(tool?.description).toContain("Resume EXISTING para sessions");
    expect(tool?.inputSchema.properties).toHaveProperty('session');
    expect(tool?.inputSchema.properties).toHaveProperty('prompt');
    expect(tool?.inputSchema.properties).toHaveProperty('file');
  });

  test('should define para_config_set tool with correct schema', () => {
    const tool = PARA_TOOLS.find(t => t.name === 'para_config_set');
    expect(tool).toBeTruthy();
    expect(tool?.name).toBe("para_config_set");
    expect(tool?.description).toContain("Set para configuration values");
    expect(tool?.inputSchema.required).toEqual(["path", "value"]);
    expect(tool?.inputSchema.properties?.value?.oneOf).toHaveLength(3);
  });

  test('should define all required tools', () => {
    const expectedTools = [
      'para_start',
      'para_finish',
      'para_resume',
      'para_list',
      'para_recover',
      'para_config_show',
      'para_config_set',
      'para_cancel',
      'para_status_show'
    ];
    
    expectedTools.forEach(toolName => {
      const tool = PARA_TOOLS.find(t => t.name === toolName);
      expect(tool).toBeTruthy();
      expect(tool?.name).toMatch(/^para_/);
    });
  });
});