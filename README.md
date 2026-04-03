# jig

Ask an LLM to generate a unit test for `BookingService` three times and you'll get three different file structures, three import styles, and three naming conventions. The code will be fine. It'll probably pass a review. But the LLM spent valuable tokens re-deriving patterns that should have been fixed before it ever started writing.

This is the consistency problem. Not whether agents can write code — they can — but whether they write it the same way every time. In a codebase where six engineers have been shipping for two years, "correct but inconsistent" is a bug.

jig fixes this.

## What It Is

A jig in manufacturing is a guide that holds a part in place while you cut it. It doesn't make the part. It makes sure the part comes out right.

jig is a CLI. It takes a YAML recipe and a JSON object of variables, renders Jinja2 templates, and writes files. One binary, zero runtime dependencies, deterministic output. Same recipe plus same variables equals same files, every time.

The insight: LLMs are good at understanding intent and extracting variables from existing code. Templates are good at producing consistent, boilerplate-heavy files from those variables. Right now, agents do both — they figure out what you want AND they generate the code from scratch every time. jig splits that work. The LLM handles the judgment. jig handles the mechanical output.

## A Recipe

```yaml
name: unit-test
description: Generate a pytest unit test for a Python class

variables:
  module:
    type: string
    required: true
  class_name:
    type: string
    required: true
  methods:
    type: array
    items: string
    default: []

files:
  - template: test.py.j2
    to: "tests/{{ module | replace('.', '/') }}/test_{{ class_name | snakecase }}.py"

  - template: conftest_fixture.j2
    inject: "tests/conftest.py"
    after: "^# fixtures"
    skip_if: "{{ class_name }}"
```

Variables in, files out. The recipe declares what it needs. The `files` section says what to do: create a new test file, then inject a fixture into the existing conftest. The `skip_if` line makes it idempotent — run it twice, get one fixture.

An agent reads `BookingService.py`, figures out the module path, class name, and public methods, then passes that JSON to jig. jig renders the templates and puts the files in the right place. Neither does the other's work.

## The Real Work Is Brownfield

Scaffolding new files is the easy case. The hard case — the everyday case — is extending existing code. You already have the model, the service, the schema, the admin, the tests. You're adding a field. It touches six files. Each change is mechanical: find the right block, add one more entry in the established pattern.

```yaml
- template: model_field.j2
  patch: "{{ app }}/models/{{ model | snakecase }}.py"
  anchor:
    pattern: "^class {{ model }}\\("
    scope: class_body
    position: after_last_field
  skip_if: "{{ field_name }}"
```

The anchor system combines a regex to find the class, a scope to identify the class body, and a position to insert after the last field. No hardcoded line numbers. No full AST parser. Lightweight heuristics that cover the vast majority of real-world code.

One recipe chains six patches. "Add a field to Reservation" becomes one command:

```bash
jig run add-field/recipe.yaml \
  --vars '{"app":"hotels","model":"Reservation","field_name":"loyalty_tier","field_type":"CharField","field_args":"max_length=50","nullable":true}'
```

Six files patched. The LLM's contribution was understanding the user's intent and constructing the variables. Everything else was deterministic.

## When It Fails

When a patch can't find its anchor, jig fails — but it fails with the rendered content included:

```json
{
  "action": "error",
  "error": "scope_parse_failed",
  "rendered_content": "    loyalty_tier = models.CharField(max_length=50, null=True)"
}
```

The deterministic part is never wasted by a placement failure. The agent reads the error, falls back to its native Edit tool, and places the content manually. jig did the mechanical work. The agent handles the judgment call.

This boundary is deliberate. Template rendering is deterministic and should be. Code placement sometimes requires understanding context that only an LLM has.

## Design Principles

**JSON in, files out.** No interactive prompts. An LLM produces JSON naturally; jig consumes it.

**Deterministic.** Same recipe, same variables, same existing files, same output. No randomness, no creativity, no drift.

**Idempotent.** Every operation can be run twice safely. An agent retrying a failed run never duplicates content.

**Transparent failures.** Every error says what, where, and why. Exit codes are semantic. Agents branch on exit codes without parsing error messages.

**Composable with LLM tools.** jig handles what templates handle well: boilerplate, structure, repetition. It doesn't try to handle what LLMs handle well: understanding semantics, adapting to unexpected structures, choosing the right insertion point in novel code. The boundary between jig's work and the agent's work is the design.

## Status

Pre-release. The [full specification](jig.md) is complete. Implementation is underway in Rust — single binary, installable via cargo.
