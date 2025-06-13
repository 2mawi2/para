# MCP Setup Investigation and Fix

## Problem
The MCP setup (`para init mcp`) is not working correctly across different projects:
1. Claude doesn't automatically recognize the MCP server in other projects
2. The `mcp.json` file gets created but no `.gitignore` entry is added (should be default)
3. The MCP server itself seems to be working but Claude can't connect to it
4. Para dispatch command sometimes fails and waits indefinitely without starting agents

## Tasks to Complete

### 1. Investigate MCP Setup Process
- Examine the `para init mcp` command implementation
- Review how the MCP server configuration is generated
- Check if there are any missing steps in the setup process

### 2. Research Claude MCP Recognition Issues
- Investigate why Claude doesn't automatically detect the MCP server
- Check if there are configuration issues with the generated `mcp.json`
- Verify the MCP server is properly installed and accessible
- Look into Claude's MCP server discovery mechanism

### 3. Fix .gitignore Integration
- Ensure `mcp.json` is automatically added to `.gitignore` during `para init mcp`
- This should be the default behavior to prevent accidental commits of local MCP config

### 4. Fix Dispatch Reliability Issues
- Investigate why para dispatch sometimes hangs indefinitely
- Ensure agents start properly and don't get stuck in waiting state
- Improve error handling and timeout mechanisms for dispatch operations

### 5. Test and Verify
- Test the MCP setup in a fresh project
- Verify Claude can properly recognize and connect to the MCP server
- Ensure the setup works consistently across different environments
- Test dispatch reliability with multiple agents

## Expected Deliverables
- Fixed MCP setup process that works reliably
- Automatic `.gitignore` entry creation for `mcp.json`
- Clear documentation of any additional setup steps needed
- Verification that Claude can recognize the MCP server automatically

When complete, run: para integrate "Fix MCP setup issues and improve reliability"