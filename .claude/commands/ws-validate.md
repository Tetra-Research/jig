# Workstream Validate

Check task readiness for merge: run tests and verify VALIDATION.md coverage.

## Usage
```
/ws-validate [workstream] [task]
```

## Arguments
$ARGUMENTS

---

Run the validation script:

```bash
./scripts/validate.sh $ARGUMENTS
```

Checks:
1. Test suite passes (auto-detects just/cargo/npm)
2. VALIDATION.md has no FAIL, MISSING, or PENDING entries
3. Outputs READY or NOT READY recommendation
