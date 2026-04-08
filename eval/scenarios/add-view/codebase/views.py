from rest_framework.decorators import api_view
from rest_framework.response import Response
from .models import Entity


@api_view(["GET"])
def entity_list(request):
    entities = Entity.objects.all()
    return Response([{"id": r.id, "display_name": r.display_name} for r in entities])


@api_view(["GET"])
def entity_detail(request, pk):
    entity = Entity.objects.get(pk=pk)
    return Response({
        "id": entity.id,
        "display_name": entity.display_name,
        "unit": entity.unit_number,
        "status": entity.status,
    })
