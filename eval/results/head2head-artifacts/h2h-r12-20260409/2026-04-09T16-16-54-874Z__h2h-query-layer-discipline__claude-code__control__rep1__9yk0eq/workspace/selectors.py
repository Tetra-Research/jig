from .models import Entity


def select_active_entities():
    return (
        Entity.objects.active()
        .select_related()
    )
