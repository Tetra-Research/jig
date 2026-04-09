from .selectors import select_active_entities


def entity_list(request):
    records = select_active_entities()
    return {"results": []}
