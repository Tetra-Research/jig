from rest_framework.decorators import api_view
from rest_framework.response import Response

from .models import Entity


@api_view(["GET"])
def entity_detail(request, pk):
    return Response({"id": pk}, status=200)
