import { describe, it, expect } from "vitest";
import { translateResult } from "../src/result.js";

describe("translateResult", () => {
  it("success — passthrough stdout", () => {
    const res = translateResult("jig_run", { exitCode: 0, stdout: '{"ok":true}', stderr: "" });
    expect(res.isError).toBe(false);
    expect(res.content[0].text).toBe('{"ok":true}');
  });

  it("exit code 1 — recipe validation error", () => {
    const res = translateResult("jig_run", { exitCode: 1, stdout: "", stderr: "bad recipe" });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain("recipe validation error");
    expect(res.content[0].text).toContain("bad recipe");
  });

  it("exit code 1 — includes stdout when non-empty", () => {
    const res = translateResult("jig_run", { exitCode: 1, stdout: '{"detail":"extra"}', stderr: "bad recipe" });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain("bad recipe");
    expect(res.content[0].text).toContain('{"detail":"extra"}');
  });

  it("exit code 2 — template rendering error", () => {
    const res = translateResult("jig_render", { exitCode: 2, stdout: "", stderr: "render fail" });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain("template rendering error");
  });

  it("exit code 3 — with rendered_content in operations", () => {
    const json = JSON.stringify({
      operations: [{ rendered_content: "pub struct Foo;" }],
    });
    const res = translateResult("jig_run", { exitCode: 3, stdout: json, stderr: "file op fail" });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain("file operation error");
    expect(res.content[0].text).toContain("Rendered content (for manual fallback):");
    expect(res.content[0].text).toContain("pub struct Foo;");
  });

  it("exit code 3 — no parseable JSON", () => {
    const res = translateResult("jig_run", { exitCode: 3, stdout: "not json", stderr: "fail" });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain("file operation error");
    expect(res.content[0].text).not.toContain("Rendered content");
  });

  it("exit code 4 — variable validation error", () => {
    const res = translateResult("jig_run", { exitCode: 4, stdout: "", stderr: "bad var" });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain("variable validation error");
  });

  it("binary not found (ENOENT)", () => {
    const res = translateResult("jig_run", { exitCode: -1, stdout: "", stderr: "spawn jig ENOENT" });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain("jig binary not found");
  });

  it("spawn permission denied", () => {
    const res = translateResult("jig_run", { exitCode: -1, stdout: "", stderr: "EACCES permission denied" });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain("EACCES");
  });

  it("timeout", () => {
    const res = translateResult("jig_run", { exitCode: -2, stdout: "", stderr: "jig command timed out after 30 seconds" });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain("timed out");
  });

  it("jig_render --to success — confirmation message", () => {
    const res = translateResult("jig_render", { exitCode: 0, stdout: "", stderr: "" }, { to: "/tmp/out.rs" });
    expect(res.isError).toBe(false);
    expect(res.content[0].text).toBe("Rendered template to /tmp/out.rs");
  });

  it("jig_render without --to — returns rendered content", () => {
    const res = translateResult("jig_render", { exitCode: 0, stdout: "pub struct Foo;", stderr: "" });
    expect(res.isError).toBe(false);
    expect(res.content[0].text).toBe("pub struct Foo;");
  });

  it("determinism — same input gives same output", () => {
    const input = { exitCode: 0, stdout: '{"x":1}', stderr: "" };
    const a = translateResult("jig_run", input);
    const b = translateResult("jig_run", input);
    expect(a).toEqual(b);
  });
});
