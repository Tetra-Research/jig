# Workstream Review

Adversarial code review channeling multiple expert perspectives.

## Usage
```
/ws-review <workstream> [task]
```

## Arguments
$ARGUMENTS

---

## Review Perspectives

Channel the rigor of:
- **Correctness** — Does the code do what SPEC.md says? Error handling complete?
- **Simplicity** — Essential complexity only? No unnecessary abstractions?
- **Debuggability** — Can you diagnose a failure at 3am with these logs and errors?
- **API ergonomics** — Is the public interface pleasant and hard to misuse?
- **Spec alignment** — Does implementation match EARS acceptance criteria exactly?

## Review Process

1. **Load context** — Read SPEC.md, PLAN.md, SHARED-CONTEXT.md, INVARIANTS.md
2. **Fresh eyes pass** — Read the code without spec context first. What jumps out?
3. **Correctness review** — Walk through each EARS criterion. Is it implemented correctly?
4. **Design review** — Right abstractions? Right boundaries?
5. **Consistency review** — Matches INVARIANTS.md constraints? ARCHITECTURE.md patterns?
6. **Testing review** — Tests cover EARS criteria? Edge cases handled?
7. **LLM-specific traps** — Hallucinated APIs? Silent failures? Unnecessary complexity?

## Output

Structure as:

**Verdict:** Approve / Request Changes / Needs Discussion

**Critical** — Must fix before merge
**Major** — Should fix
**Minor** — Nice to fix

Save review to `docs/workstreams/<ws>/reviews/<timestamp>.md`
