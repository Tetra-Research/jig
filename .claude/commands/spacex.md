# SpaceX Analysis

Apply SpaceX's 5-step "Algorithm" to analyze code for simplification opportunities.

## Usage

- `/spacex` - Analyze the current conversation context (recent files, proposed changes)
- `/spacex path/to/file.rs` - Analyze a specific file or directory

## Instructions

Run The Algorithm in order. For each step, provide concrete findings.

### Step 1: Question Requirements

Identify requirements, features, or code that should be challenged:
- What assumptions is this code making?
- What features might not be needed?
- What requirements are undocumented or unclear?
- Who actually uses each capability?

### Step 2: Delete

Identify candidates for deletion:
- Unused code paths, exports, or functions
- Features with no apparent users
- Abstractions with single implementations
- Config options that are never changed
- Compatibility code for deprecated systems

Be aggressive. List specific files, functions, or blocks to remove.

### Step 3: Simplify

For what remains after deletion, identify simplification opportunities:
- Abstractions that could be inlined
- Layers that could be flattened
- Generic code that could be concrete
- Custom solutions that could use libraries
- Complex patterns that could be straightforward

### Step 4: Accelerate

Identify what slows down iteration:
- Slow tests or builds
- Large, hard-to-review code areas
- Deployment friction
- Feedback loop bottlenecks

### Step 5: Automate (or De-automate)

Evaluate current automation:
- Is there automation that should exist?
- Is there automation hiding problems?
- Is there premature automation that should be manual first?

## Output Format

```
## SpaceX Analysis: [target]

### Idiot Index Assessment
[Ratio of complexity to value - high/medium/low with reasoning]

### Step 1: Question
- [Requirement/assumption to challenge]
- [Requirement/assumption to challenge]

### Step 2: Delete
- [ ] `path/to/file.rs:function_name` - [reason]
- [ ] `path/to/feature/` - [reason]

### Step 3: Simplify
- [ ] [Simplification opportunity with before/after sketch]

### Step 4: Accelerate
- [ ] [Feedback loop improvement]

### Step 5: Automate
- [ ] [Automation recommendation or warning]

### Priority Actions
1. [Most impactful change]
2. [Second most impactful]
3. [Third most impactful]
```

## Arguments

$ARGUMENTS - Optional file or directory path to analyze. If omitted, analyze recent conversation context.
