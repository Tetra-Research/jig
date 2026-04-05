import { describe, it, expect, afterEach } from "vitest";
import { findJigBinary, getJigVersion } from "../src/binary.js";

describe("findJigBinary", () => {
  const originalEnv = process.env["JIG_PATH"];

  afterEach(() => {
    if (originalEnv !== undefined) {
      process.env["JIG_PATH"] = originalEnv;
    } else {
      delete process.env["JIG_PATH"];
    }
  });

  it("finds jig on PATH", () => {
    const result = findJigBinary();
    expect(result).toBeTruthy();
    expect(result).toContain("jig");
  });

  it("JIG_PATH env override", () => {
    process.env["JIG_PATH"] = "/custom/path/jig";
    expect(findJigBinary()).toBe("/custom/path/jig");
  });

  it("CLI path takes precedence over JIG_PATH", () => {
    process.env["JIG_PATH"] = "/env/path/jig";
    expect(findJigBinary("/explicit/path/jig")).toBe("/explicit/path/jig");
  });

  it("returns null when not found", () => {
    delete process.env["JIG_PATH"];
    // Override PATH to ensure jig isn't found
    const origPath = process.env["PATH"];
    process.env["PATH"] = "/nonexistent";
    try {
      expect(findJigBinary()).toBeNull();
    } finally {
      process.env["PATH"] = origPath;
    }
  });
});

describe("getJigVersion", () => {
  it("returns version string", () => {
    const path = findJigBinary();
    if (!path) return; // skip if jig not installed
    const version = getJigVersion(path);
    expect(version).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it("returns null for invalid binary", () => {
    expect(getJigVersion("/nonexistent/jig")).toBeNull();
  });
});
