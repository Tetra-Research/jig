from rest_framework.decorators import api_view
from rest_framework.response import Response

from .models import Entity
from .schemas import EntitySummaryRequest, EntitySummaryResponse
from .services import build_entity_summary


@api_view(["GET"])
def entity_detail(request, pk):
    return Response({"id": pk}, status=200)


@api_view(["POST"])
def entity_summary(request, pk):
    request_contract = EntitySummaryRequest(data=request.data)
    request_contract.is_valid(raise_exception=True)
    payload = request_contract.validated_data

    result = build_entity_summary(pk=pk, payload=payload)
    response_contract = EntitySummaryResponse(result)
    return Response(response_contract.data, status=200)
