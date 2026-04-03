# Software Engineering Parallels

How The Algorithm maps to established software engineering principles.

## Step 1: Question Requirements -> Challenge Assumptions

### Related Principles

**YAGNI (You Aren't Gonna Need It)** - Kent Beck, Extreme Programming
> "Implement features when they are actually needed, not when you foresee that you might need them."

Capital One product leaders "had to say 'no' or 'not yet' to about 30 features." After launch, those 30 features turned out to be irrelevant anyway.

**The XY Problem**
Users often ask for X when they actually need Y. Question the requirement to find the real problem.

### Software Questions to Ask

- Is this feature in the user's actual workflow, or imagined?
- When was this requirement last validated?
- Who will notice if we don't build this?
- Are we solving a real problem or a theoretical one?

---

## Step 2: Delete -> Remove Code and Features

### Related Principles

**"The best code is no code at all"** - Jeff Atwood
> "Every new line of code you willingly bring into the world is code that has to be debugged, code that has to be read and understood, code that has to be supported."

> "If you love writing code--really, truly love to write code--you'll love it enough to write as little of it as possible."

**Code is Liability** - Rich Skrenta
> "Code is our enemy. Code is bad. It rots. It requires periodic maintenance. It has bugs that need to be found. New features mean old code has to be adapted."

### What to Delete

- Features with zero or near-zero usage
- Abstractions with only one implementation
- "Framework" code that wraps one library
- Config options nobody changes
- Compatibility shims for deprecated systems
- Dead code paths
- Unused exports and public APIs

### The 10% Rule

If you're not adding back at least 10% of what you delete, you didn't delete enough. Be aggressive. You can always restore from git.

---

## Step 3: Simplify -> Reduce Complexity

### Related Principles

**KISS (Keep It Simple, Stupid)**
> "Most systems work best if they are kept simple rather than made complicated."

**Premature Optimization** - Donald Knuth
> "We should forget about small efficiencies, say about 97% of the time: premature optimization is the root of all evil."

The corollary for abstraction: premature abstraction is also evil. Don't abstract until you have 3+ concrete cases.

### Simplification Tactics

| Complex | Simple |
|---------|--------|
| Inheritance hierarchy | Composition or plain functions |
| Plugin system | Direct implementation |
| Config file | Hardcoded value |
| Microservice | Module in monolith |
| Custom solution | Library |
| Generic<T, U, V> | Concrete types |
| 5 layers | 2 layers |

### The Rule of Three

Don't abstract until you've written the same thing three times. Two similar things might be coincidence. Three is a pattern.

---

## Step 4: Accelerate -> Faster Feedback

### Related Principles

**Continuous Integration**
Integrate frequently, test continuously, deploy often.

**Small Batches**
Smaller changes = faster review, easier debugging, quicker deploys.

### Acceleration Tactics

- Reduce test suite runtime (delete slow/flaky tests first per Step 2)
- Smaller PRs (under 400 lines)
- Feature flags for incremental rollout
- Local development that matches production
- Fast, focused unit tests over slow integration tests

### Warning

> "If you're digging your grave, don't dig it faster."

Don't accelerate a bad process. Simplify first.

---

## Step 5: Automate -> Scripts, CI/CD, Tooling

### Related Principles

**Automation Should Be Last**
> "The big mistake was that I began by trying to automate every step."

Automating a broken process just makes it break faster and harder to debug.

### When to Automate

- You've done the task manually many times
- The process is stable and well-understood
- Bugs have been shaken out
- The value is proven

### When NOT to Automate

- You're still figuring out the process
- Requirements are changing rapidly
- The process might be deleted (Step 2)
- Automation would hide problems

---

## Anti-Patterns by Step

### Step 1 Violations (Unquestioned Requirements)

- **Cargo culting**: "We do it because everyone does"
- **Resume-driven development**: Using tech because it looks good
- **Speculative generality**: "We might need this someday"

### Step 2 Violations (Failure to Delete)

- **Dead code accumulation**: "Might need it later"
- **Feature creep**: Adding without removing
- **Backward compatibility forever**: Never deprecating

### Step 3 Violations (Over-Complexity)

- **Premature abstraction**: Abstract before concrete
- **Astronaut architecture**: Over-designed systems
- **Gold plating**: Adding unnecessary polish

### Step 4 Violations (Slow Cycles)

- **Big bang releases**: Months between deploys
- **Mega PRs**: Thousands of lines per review
- **Test suite that takes hours**

### Step 5 Violations (Premature Automation)

- **Automating exploration**: CI for experiments
- **Complex pipelines for simple tasks**
- **Automation that hides manual understanding**

---

## The Software Idiot Index

Ratio of complexity to value delivered.

**High idiot index symptoms:**
- 10 files changed for a one-line feature
- Abstraction layers that just pass through
- Config systems more complex than the app
- Build times longer than feature development
- More test code than production code (for simple features)

**Low idiot index:**
- Change is proportional to feature size
- New developers can contribute quickly
- Debugging is straightforward
- The system does what it looks like it does

---

## Sources

- [Jeff Atwood - The Best Code is No Code](https://blog.codinghorror.com/the-best-code-is-no-code-at-all/)
- [Martin Fowler - YAGNI](https://martinfowler.com/bliki/Yagni.html)
- [Wikipedia - YAGNI](https://en.wikipedia.org/wiki/You_aren't_gonna_need_it)
- [GeeksforGeeks - Premature Optimization](https://www.geeksforgeeks.org/software-engineering/premature-optimization/)
- [Stack Overflow - Building SpaceX Software](https://stackoverflow.blog/2021/05/13/building-the-software-that-helps-build-spacex/)
