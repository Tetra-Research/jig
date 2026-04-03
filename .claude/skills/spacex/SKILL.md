---
name: spacex
description: |
  SpaceX's 5-step "Algorithm" for simplification and engineering excellence.
  Activates when:
  - Reviewing code for complexity or over-engineering
  - Planning new features, refactors, or architectural changes
  - User mentions "simplify", "reduce", "delete", "over-engineered", "complexity"
  - Before adding new abstractions, dependencies, or layers
  - Evaluating whether code/features should exist at all
---

# The Algorithm

SpaceX's 5-step engineering process, applied to software. **Order matters** - do not skip ahead.

## Step 1: Question Every Requirement

> "Requirements from smart people are the most dangerous, because people are less likely to question them."

**Ask:**

- Who requested this? (Never accept "the team" or "the spec" - get a name)
- What problem does this actually solve?
- What happens if we don't do it?
- Is this requirement still valid, or is it legacy?

**Software translation:** Challenge user stories, tickets, and "requirements" docs. Many features exist because someone once thought they might be needed.

## Step 2: Delete the Part or Process

> "If you do not end up adding back at least 10% of what you delete, you didn't delete enough."

**Ask:**

- What can we remove entirely?
- What code paths are never exercised?
- What features have no users?
- What abstractions exist "just in case"?

**Software translation:** Delete code, remove features, kill unused abstractions. The best code is no code at all.

## Step 3: Simplify and Optimize

> "The most common error of a smart engineer is to optimize something that should not exist."

**Only after steps 1 and 2.** Now simplify what remains.

**Ask:**

- Can this be inlined instead of abstracted?
- Can we use a simpler data structure?
- Can we remove a layer of indirection?
- Is there a library that does this?

**Software translation:** Refactor toward simplicity. Flatten hierarchies. Reduce moving parts.

## Step 4: Accelerate Cycle Time

> "If you're digging your grave, don't dig it faster."

**Only after steps 1-3.** Now speed up iteration.

**Ask:**

- How can we get feedback faster?
- Can we reduce PR size?
- Can we deploy more frequently?
- What's blocking quick iteration?

**Software translation:** Faster tests, smaller changes, quicker deploys. But only after simplifying.

## Step 5: Automate

> "The big mistake was that I began by trying to automate every step."

**Last, never first.** Automate the simplified, validated process.

**Ask:**

- Have we done this manually enough times to understand it?
- Is the process stable enough to automate?
- Will automation hide problems?

**Software translation:** CI/CD, scripts, tooling - but only for well-understood, simplified processes.

---

## The Idiot Index

Ratio of complexity to value delivered. High ratio = over-engineered.

**Symptoms of high idiot index:**

- Abstractions with one implementation
- Config for things that never change
- "Flexible" systems that flex one way
- Layers that just pass through
- Code "for future use"

---

## Anti-Patterns to Delete

| Anti-Pattern            | The Algorithm Says                          |
| ----------------------- | ------------------------------------------- |
| Premature abstraction   | Delete it (Step 2)                          |
| Speculative generality  | Question requirement (Step 1)               |
| Gold plating            | Delete it (Step 2)                          |
| Over-configuration      | Simplify (Step 3)                           |
| Complex build pipelines | Simplify before automating (Step 3, then 5) |

---

## Quick Reference

When reviewing code or planning features, walk through in order:

1. **Question** - Should this exist? Who needs it?
2. **Delete** - What can we remove? Be aggressive.
3. **Simplify** - Make what remains as simple as possible.
4. **Accelerate** - Speed up feedback loops.
5. **Automate** - Only now, automate the simple process.
