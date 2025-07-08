import { PARA_RESOURCES } from './resource-definitions';

describe('Resource Definitions', () => {
  test('should define current-session resource', () => {
    const resource = PARA_RESOURCES.find(r => r.uri === "para://current-session");
    expect(resource).toBeTruthy();
    expect(resource?.uri).toBe("para://current-session");
    expect(resource?.name).toBe("Current Session");
    expect(resource?.description).toBe("Information about the current para session");
    expect(resource?.mimeType).toBe("application/json");
  });

  test('should define config resource', () => {
    const resource = PARA_RESOURCES.find(r => r.uri === "para://config");
    expect(resource).toBeTruthy();
    expect(resource?.uri).toBe("para://config");
    expect(resource?.name).toBe("Para Configuration");
    expect(resource?.description).toBe("Current para configuration");
    expect(resource?.mimeType).toBe("application/json");
  });
});