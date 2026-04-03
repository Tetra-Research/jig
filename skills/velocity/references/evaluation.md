# Evaluation Heuristics

Use these lenses to evaluate whether a finding is worth acting on. Not every lens applies to every finding — use the ones that fit.

## Convention Over Configuration

The strongest DX improvements establish a convention so developers don't have to make decisions.

- Is there an obvious "right way" to do this that isn't documented?
- Are developers making the same choice independently? That's a convention waiting to be codified.
- Could a skill or rule make the default path the correct path?

**Good sign:** "Everyone does it this way but nobody wrote it down."
**Bad sign:** "There are legitimate reasons to do it differently in different contexts."

## Optimize for Frequency

The 10-minute task that happens 100 times matters more than the 2-hour task that happens once.

- How often does this workflow / pattern occur across the team?
- Is this something every developer hits, or just specialists?
- Would automating this free up meaningful time or just save seconds?

**Estimate:** `frequency x time_per_occurrence x number_of_developers`. If the total is under an hour per quarter, probably not worth a skill.

## Consistency Is the Product

Inconsistency creates cognitive load. Every time a developer encounters "the same thing done differently," they have to stop and figure out which way is right.

- Are there multiple ways to do the same thing in the codebase?
- Does the inconsistency cause real confusion, or is it cosmetic?
- Would a rule or CLAUDE.md entry prevent drift going forward?

**Worth acting on:** Different error handling patterns across similar API endpoints.
**Not worth acting on:** Slightly different variable naming in files last touched 3 years ago.

## Progressive Disclosure

Good abstractions make simple things simple and complex things possible.

- Does the proposed extension help with the common case without blocking edge cases?
- Would this make onboarding easier for new developers?
- Can someone who doesn't know about the extension still do their job?

## When NOT to Abstract

- Pattern has appeared fewer than 3 times
- The abstraction would require more code than the duplication
- Future requirements are unclear and the abstraction would constrain them
- The "duplication" is conceptually different code that happens to look similar
- Adding the abstraction requires touching many files for minimal gain
- The "pattern" is just the framework being explicit (not all boilerplate is bad)

## Generators Over Instructions

The core insight: **just because Claude can write code doesn't mean we want Claude to write it.**

Claude and other LLMs can produce correct code given a good reference — that's not in doubt. The real constraint is the context window. Every line Claude generates from scratch is context consumed: reading the reference, reasoning about the template, writing boilerplate, verifying it matches. A generator script does all of that in zero tokens. Claude's context is then free for the parts that actually require reasoning.

This means generators are worth building even when they have many arguments and heavy customization. A script with 15 flags that scaffolds 200 lines of boilerplate is still saving significant context and tokens compared to Claude reading a reference file, reproducing the pattern, and hoping it doesn't drift. The script runs in milliseconds and is deterministic. Claude writing it takes tokens and is probabilistic.

**When to reach for a generator:**

Any repeating coding pattern where the structure is predictable but the details vary. Look for:

- "Every time we add a new [X], we copy an existing one and change the names"
- "The boilerplate is always the same, but people forget [specific part]"
- "New developers don't know which file to copy from"
- Files that share 70%+ structure across instances — even if the remaining 30% has real variation, the 70% should be generated

**What generators buy you:**

- **Context window savings.** The scaffolded boilerplate costs zero tokens. Claude's context is reserved for domain logic, edge cases, and the parts that actually need thought.
- **Deterministic output.** No drift, no forgotten imports, no style inconsistencies in the template portions.
- **Compounding speed.** Each generator makes the next task faster — not just for Claude, but for developers who can run them without Claude at all.

**Generators don't have to be complete.** A generator that scaffolds 60% of the file and leaves `# TODO` markers for the domain-specific parts is a great getting-started motion. The goal isn't to eliminate Claude's involvement — it's to skip the predictable parts so Claude (or the developer) starts from a working skeleton instead of a blank file. Partial generators are still high-value generators.

**Structure:** A generator skill has a `scripts/` directory with the scaffolding logic and a `templates/` directory with the file templates. The SKILL.md orchestrates: ask what to name it, run the script, then guide Claude to fill in the blanks.

## Evaluating Extension Type

| Signal | Extension |
|--------|-----------|
| "Claude keeps doing X wrong in these files" | **Rule** (path-scoped) |
| "Everyone should know this but nobody does" | **CLAUDE.md** addition |
| "I do these 5 steps every time I need to..." | **Skill** |
| "Every time we add a new X, we copy and modify Y" | **Generator skill** (skill + script + templates) |
| "This needs a specialist perspective" | **Agent** |
| "We should block X from ever happening" | **Hook** |
| "Claude needs data from [external system]" | **MCP Server** |

When in doubt, start with the lightest-weight option (CLAUDE.md > rule > skill > generator > agent > hook > MCP).
