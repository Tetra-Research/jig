# Landing Page Synthesis

This document captures what the strongest library and framework sites are doing well, what patterns repeat across them, and how those patterns should shape the public landing page for `jig`.

## Reference Sites

Primary references:
- `https://biomejs.dev/`
- `https://bun.sh/`
- `https://tailwindcss.com/`
- `https://astro.build/`
- `https://ui.shadcn.com/`
- `https://zod.dev/`
- `https://www.prisma.io/`

These sites are not equally relevant. For `jig`, the strongest references are:
- `Biome`: closest product shape and strongest demo pattern
- `Bun`: strongest proof and benchmark framing
- `Tailwind CSS`: strongest example-driven visual communication
- `Astro`: strongest information discipline
- `Zod`: strongest concise canonical example

## What Works Across All Of Them

### 1. They explain the product immediately
The best sites make the first screen answer three questions without scrolling:
- what is this
- who is it for
- why should I care

They do not make the user infer the category.

For `jig`, the first screen needs to say plainly that it is:
- a deterministic file generation tool
- for coding agents and developers using them
- for repeated, shape-constrained edits that should stop being re-authored from scratch

### 2. They provide a concrete action above the fold
The strongest library sites give the visitor something executable immediately:
- install command
- create command
- minimal usage command
- clear CTA to GitHub or docs

This matters because good library sites do not just describe capability. They let the reader picture first use instantly.

For `jig`, that means the hero should include:
- install command
- one `jig run` command
- a GitHub link
- a pointer to examples

### 3. They demonstrate, not just describe
Every strong site has a concrete artifact near the top:
- code example
- interactive demo
- before/after transformation
- API snippet
- benchmark output

The common pattern is simple: the product proves itself with an artifact, not a paragraph.

For `jig`, the most natural artifact is:
- `recipe.yaml`
- `vars.json`
- before code
- after code
- resulting diff

This is closer to Biome and Tailwind than to a typical docs homepage.

### 4. They compress the value proposition into a small number of ideas
Good sites do not sell twelve ideas. They usually sell three:
- speed
- correctness
- developer experience

Or some equivalent version of that trio.

For `jig`, the core value pillars should stay narrow:
- deterministic output for repetitive edits
- lower agent cost and token usage on routine work
- more trustworthy autonomous execution on shape-constrained tasks

### 5. They are opinionated about the ideal workflow
The good sites do not leave workflow ambiguous. They show the happy path.

Examples:
- Bun shows the install command and what it replaces
- Astro shows how to get started and what kind of project it enables
- shadcn/ui frames itself as a foundation you adapt
- Zod shows the canonical declaration pattern immediately

For `jig`, the site should clearly imply this flow:
1. choose or write a recipe
2. provide vars
3. run `jig`
4. let the agent reuse the pattern instead of rewriting the edit manually

### 6. They keep the top-level information architecture simple
The strongest sites do not dump everything at once.

Common traits:
- one strong hero
- one short proof/demo section
- one benefits section
- one examples section
- one quick start
- one footer with links

This is especially consistent across Astro, Biome, and Zod.

For `jig`, a single-page structure is enough right now.

### 7. They use proof in a format that fits the product
Different sites prove value differently:
- Bun uses benchmarks
- Tailwind uses visual examples
- Zod uses a tiny code sample
- Biome uses code transformation
- Prisma uses ecosystem breadth and trust signals

The common rule is not "use benchmarks" or "use examples." The common rule is:
- use proof that matches the product's actual claim

For `jig`, the right proof is:
- before/after code transformations
- exact recipe examples
- head-to-head routine-task results

### 8. They avoid broad vague slogans unless the next section grounds them immediately
Strong marketing copy may be broad, but the page grounds it quickly in concrete output.

That means `jig` can use strong copy in the hero, but the next screen must prove it with code.

### 9. They speak in the language of the user’s workflow
The best sites describe value in operational terms:
- fewer steps
- faster execution
- better safety
- less reinvention
- stronger defaults

For `jig`, the language should stay close to:
- repeated edits
- templates
- patches
- predictable code generation
- agents staying on the rails

### 10. They make the next step obvious
A good library landing page always answers:
- where do I start
- what do I click next
- what is the smallest successful path

For `jig`, that means the page should always surface:
- install
- quick start
- examples
- GitHub
- agent integrations

## What Each Reference Contributes

### Biome
What works:
- transformation-centric framing
- code-first proof
- strong visual emphasis on input and output

What to borrow:
- a before/after section showing a real repeated edit pattern
- a product explanation that feels mechanical, not fluffy

### Bun
What works:
- immediate install and command visibility
- strong confidence in performance claims
- direct workflow replacement framing

What to borrow:
- clear install path in the hero
- a compact proof section with measured eval outcomes

### Tailwind CSS
What works:
- product understanding through examples instead of long prose
- visual density without losing hierarchy

What to borrow:
- example cards that make recipes feel tangible
- a page that rewards scrolling with real artifacts, not generic illustrations

### Astro
What works:
- disciplined structure
- restrained copy
- clean hierarchy

What to borrow:
- simple one-page architecture
- one thesis, three pillars, one start path

### shadcn/ui
What works:
- strong product voice for developers
- emphasis on foundation and adaptation rather than lock-in

What to borrow:
- positioning `jig` as a deterministic layer you can build on
- copy that feels toolmaker-oriented rather than corporate

### Zod
What works:
- concise canonical example
- immediate clarity about what the library does

What to borrow:
- one tiny minimal example near the top of the page
- low-ceremony explanation of the core primitive

### Prisma
What works:
- production polish
- trust and ecosystem framing

What to borrow carefully:
- proof/trust sections later on the page
- not the overall complexity or SaaS-style sprawl

## What `jig` Should Emphasize

The landing page should focus on a small, defensible thesis:

`jig` gives coding agents a deterministic way to apply repeated code patterns without re-deriving the same edits from scratch.

That thesis breaks into four concrete claims:
- repeated edits should be recipes, not fresh reasoning every time
- deterministic generation improves consistency on shape-constrained tasks
- lower token and cost usage matters when agents run unattended
- examples and integrations matter more than a large docs surface

## Recommended One-Page Structure

### 1. Hero
Must include:
- product name and mark
- one-sentence thesis
- short subtitle about coding agents and repeated edits
- install command
- GitHub CTA
- examples CTA

### 2. Product Demo
Show one complete loop:
- recipe
- vars
- before code
- after code
- tiny explanation of why this should be deterministic

This is the most important section after the hero.

### 3. Why `jig`
Three pillars only:
- deterministic output
- lower agent spend on routine edits
- better unattended execution for repeated patterns

### 4. Example Patterns
Link to repo examples:
- add service test
- structured logging
- view contract
- query layer discipline
- schema migration safety

### 5. Agent Integrations
Explain the intended role with:
- Claude
- Codex
- local skill/plugin workflows

The framing should be:
- use `jig` when a task has a repeatable output shape
- let the agent use recipes instead of manually rebuilding known patterns

### 6. Quick Start
Keep it small:
- install
- minimal recipe
- minimal vars
- `jig run`
- expected result

### 7. Proof / Evidence
Keep this compact and honest:
- routine backend tasks
- head-to-head control vs jig skills
- equal correctness in the clean replicated set
- lower median tokens/cost/time in most scenarios
- explicit note that not every scenario is a win

### 8. Footer Links
Only a few:
- GitHub
- examples
- releases/install
- integrations

## What To Avoid

Avoid these patterns on the `jig` site:
- large docs-first navigation
- enterprise SaaS framing
- vague AI-general claims
- feature grids without code artifacts
- decorative sections that do not teach the product
- benchmarking claims without clear scope

## Tone Guidance

The tone should be:
- exact
- developer-native
- confident without hype
- specific about scope

The page should not claim that `jig` helps with all coding.
It should claim that `jig` is effective for routine, pattern-heavy, shape-constrained edits where agents otherwise waste tokens re-deriving the same output.

## Bottom Line

Across the best library sites, the common success pattern is:
- explain the tool immediately
- show the command immediately
- prove the claim with real artifacts
- keep the page structure disciplined
- make the next step obvious

For `jig`, that means the landing page should behave less like a docs portal and more like:
- a product thesis
- a code transformation demo
- a compact quick start
- a gateway into examples and agent integrations
