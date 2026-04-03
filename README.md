# jig

A template rendering CLI purpose-built for LLM code generation workflows.

A jig is a manufacturing tool that guides the shape of a part -- it doesn't make the part itself, it ensures the part comes out right every time. That's what this tool does for code generation: it turns a template + variables into files, deterministically, so an LLM doesn't have to reinvent boilerplate from scratch on every invocation.

## The Problem

LLMs are powerful code generators, but they have a consistency problem. Ask one to generate a unit test three times and you'll get three different file structures, import styles, and naming conventions. The LLM wastes context window and latency re-deriving patterns that should be fixed.

## The Gap

Existing scaffolding tools weren't designed for this:

- **Hygen** -- stale, designed for human-interactive prompts, no structured JSON input, no multi-file recipes
- **Cookiecutter / Copier** -- heavy Python dependencies, project-level scaffolding, not fine-grained file generation within an existing codebase
- **Yeoman** -- massive framework overhead, generators are full npm packages
- **NX Generators** -- coupled to the NX build system, can't be used standalone
- **envsubst / sed** -- no conditionals, no loops, no injection logic, no recipe concept

## What jig Does

1. Accepts variables as structured JSON (how LLMs naturally produce data)
2. Renders Jinja2 templates with real control flow (conditionals, loops, filters)
3. Creates new files AND injects/patches into existing files in a single operation
4. Groups multiple file operations into composable recipes
5. Ships as a single, fast Rust binary with zero runtime dependencies
6. Designed from day one to be called by an LLM, not a human at a terminal

## How It Works

An LLM reads your code, extracts context, and constructs a JSON variables object. jig takes that JSON + a recipe and deterministically renders the right files every time. If something goes wrong, jig returns structured errors with rendered content so the LLM can fall back to manual edits.

```
LLM understands intent --> extracts variables --> jig renders templates --> deterministic output
```

See [jig.md](jig.md) for the full specification.
