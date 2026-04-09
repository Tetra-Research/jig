# Control Profile

Use only control skills from `.claude/skills`.
When a prompt names a control skill, implement the skill by editing files in the repository.
Treat each control skill as an execution spec, not as a checklist review.
Do not reply with plan-only, checklist-only, or review-only output unless the prompt explicitly asks for analysis.
Do not run `jig` unless the prompt explicitly requires it.
