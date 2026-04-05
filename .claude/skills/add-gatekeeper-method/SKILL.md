---
name: add-gatekeeper-method
description: Add a resource lookup method to a gatekeeper (get_reservation, get_note, etc.)
allowed-tools: Read Bash Grep Glob Edit
argument-hint: <gatekeeper_file> <resource_name>
---

## Recipe variables

```!
jig vars ${CLAUDE_SKILL_DIR}/recipe.yaml
```

## Steps

1. Read the gatekeeper file at $0 to understand:
   - The class name and existing methods
   - The UserType enum (HOTEL_USER, UNAUTHENTICATED_GUEST)
   - What resources it already looks up (pattern: `get_<resource>`)
2. Read the model for $1 to understand:
   - The lookup field (usually UUID)
   - Relations to Hotel (direct FK or through another model)
   - What NotFound exception to raise
3. Construct the method bodies following the existing pattern:
   - Hotel user: get hotel first, then lookup resource scoped to hotel
   - Guest (if supported): similar but with guest-level auth
4. Run jig:
   ```
   jig run ${CLAUDE_SKILL_DIR}/recipe.yaml \
     --vars '{ ... }' --json --dry-run
   ```
5. Review and apply. Add any new imports (model, structlog) via Edit.

## Pattern

Every gatekeeper resource method follows this shape:
```python
def get_<resource>(self, *, hotel_slug: str, uuid: UUID) -> Model:
    if self.type == self.UserType.HOTEL_USER:
        return self._get_<resource>_for_hotel_user(hotel_slug, uuid)
    elif self.type == self.UserType.UNAUTHENTICATED_GUEST:
        return self._get_<resource>_for_unauthenticated_guest(hotel_slug, uuid)

def _get_<resource>_for_hotel_user(self, hotel_slug, uuid):
    hotel = self.get_hotel(slug_name=hotel_slug)
    try:
        obj = Model.objects.get(uuid=uuid, hotel=hotel)
        structlog.contextvars.bind_contextvars(**obj.structlog_log_context())
        return obj
    except Model.DoesNotExist:
        raise NotFound()
```
