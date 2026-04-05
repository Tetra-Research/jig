import { describe, it, expect } from "vitest";
import {
  buildRunArgs,
  buildValidateArgs,
  buildVarsArgs,
  buildRenderArgs,
  buildWorkflowArgs,
} from "../src/args.js";

describe("buildRunArgs", () => {
  it("minimal — recipe only", () => {
    expect(buildRunArgs({ recipe: "r.yaml" })).toEqual(["run", "r.yaml", "--json"]);
  });

  it("with vars", () => {
    const args = buildRunArgs({ recipe: "r.yaml", vars: { key: "val" } });
    expect(args).toContain("--vars");
    expect(args[args.indexOf("--vars") + 1]).toBe('{"key":"val"}');
  });

  it("undefined vars — no --vars flag", () => {
    const args = buildRunArgs({ recipe: "r.yaml" });
    expect(args).not.toContain("--vars");
  });

  it("empty vars object — no --vars flag", () => {
    const args = buildRunArgs({ recipe: "r.yaml", vars: {} });
    expect(args).not.toContain("--vars");
  });

  it("all flags", () => {
    const args = buildRunArgs({
      recipe: "r.yaml",
      vars: { a: 1 },
      dry_run: true,
      force: true,
      base_dir: "/tmp",
      verbose: true,
    });
    expect(args).toContain("--json");
    expect(args).toContain("--dry-run");
    expect(args).toContain("--force");
    expect(args).toContain("--verbose");
    expect(args[args.indexOf("--base-dir") + 1]).toBe("/tmp");
  });
});

describe("buildValidateArgs", () => {
  it("basic", () => {
    expect(buildValidateArgs({ path: "r.yaml" })).toEqual(["validate", "r.yaml", "--json"]);
  });
});

describe("buildVarsArgs", () => {
  it("basic", () => {
    expect(buildVarsArgs({ path: "r.yaml" })).toEqual(["vars", "r.yaml"]);
  });
});

describe("buildRenderArgs", () => {
  it("minimal", () => {
    expect(buildRenderArgs({ template: "t.j2" })).toEqual(["render", "t.j2"]);
  });

  it("with --to", () => {
    const args = buildRenderArgs({ template: "t.j2", to: "out.py" });
    expect(args).toContain("--to");
    expect(args[args.indexOf("--to") + 1]).toBe("out.py");
  });

  it("with vars", () => {
    const args = buildRenderArgs({ template: "t.j2", vars: { x: 1 } });
    expect(args).toContain("--vars");
    expect(args[args.indexOf("--vars") + 1]).toBe('{"x":1}');
  });
});

describe("buildWorkflowArgs", () => {
  it("minimal", () => {
    expect(buildWorkflowArgs({ workflow: "w.yaml" })).toEqual(["workflow", "w.yaml", "--json"]);
  });

  it("all flags", () => {
    const args = buildWorkflowArgs({
      workflow: "w.yaml",
      vars: { a: 1 },
      dry_run: true,
      force: true,
      base_dir: "/tmp",
      verbose: true,
    });
    expect(args).toContain("--json");
    expect(args).toContain("--dry-run");
    expect(args).toContain("--force");
    expect(args).toContain("--verbose");
    expect(args[args.indexOf("--base-dir") + 1]).toBe("/tmp");
  });
});
