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

  it("JIG_PATH env override — valid path", () => {
    // Use a known executable for testing
    process.env["JIG_PATH"] = "/bin/sh";
    expect(findJigBinary()).toBe("/bin/sh");
  });

  it("JIG_PATH env override — invalid path returns null", () => {
    process.env["JIG_PATH"] = "/nonexistent/path/jig";
    expect(findJigBinary()).toBeNull();
  });

  it("CLI path takes precedence over JIG_PATH", () => {
    process.env["JIG_PATH"] = "/bin/sh";
    expect(findJigBinary("/usr/bin/env")).toBe("/usr/bin/env");
  });

  it("CLI path — invalid path returns null", () => {
    expect(findJigBinary("/nonexistent/jig")).toBeNull();
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
