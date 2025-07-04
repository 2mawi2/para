/**
 * Shared argument validation logic
 */

import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";

export class ArgValidator {
  // Validate that required arguments are present
  static validateRequired(args: Record<string, any>, requiredFields: string[]): void {
    for (const field of requiredFields) {
      if (!(field in args) || args[field] === undefined || args[field] === null) {
        throw new McpError(ErrorCode.InvalidRequest, `Required argument '${field}' is missing`);
      }
    }
  }

  // Validate argument types
  static validateType(value: any, expectedType: string, fieldName: string): void {
    const actualType = typeof value;
    if (actualType !== expectedType) {
      throw new McpError(
        ErrorCode.InvalidRequest, 
        `Argument '${fieldName}' must be of type ${expectedType}, got ${actualType}`
      );
    }
  }

  // Validate array arguments
  static validateArray(value: any, fieldName: string): void {
    if (!Array.isArray(value)) {
      throw new McpError(
        ErrorCode.InvalidRequest, 
        `Argument '${fieldName}' must be an array`
      );
    }
  }

  // Validate string is not empty
  static validateNonEmptyString(value: string, fieldName: string): void {
    if (typeof value !== 'string' || value.trim().length === 0) {
      throw new McpError(
        ErrorCode.InvalidRequest, 
        `Argument '${fieldName}' must be a non-empty string`
      );
    }
  }

  // Validate enum values
  static validateEnum(value: any, validValues: string[], fieldName: string): void {
    if (!validValues.includes(value)) {
      throw new McpError(
        ErrorCode.InvalidRequest, 
        `Argument '${fieldName}' must be one of: ${validValues.join(', ')}`
      );
    }
  }
}