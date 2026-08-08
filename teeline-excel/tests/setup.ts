// functions.ts references the global `CustomFunctions` object that Office.js
// normally injects at runtime. Outside Excel (here, under Vitest/Node) it
// doesn't exist, so stub the minimal shape functions.ts actually uses:
// `new CustomFunctions.Error(code, message)`.
class CustomFunctionsError extends Error {
  code: string;
  constructor(code: string, message?: string) {
    super(message);
    this.code = code;
  }
}

(globalThis as unknown as { CustomFunctions: unknown }).CustomFunctions = {
  Error: CustomFunctionsError,
  ErrorCode: {
    invalidValue: "invalidValue",
  },
};
