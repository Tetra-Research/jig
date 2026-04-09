from rest_framework import serializers


class EntityDetailResponse(serializers.Serializer):
    id = serializers.IntegerField()


class EntitySummaryRequest(serializers.Serializer):
    correlation_id = serializers.CharField(required=False, allow_blank=True)


class EntitySummaryResponse(serializers.Serializer):
    id = serializers.IntegerField()
    status = serializers.CharField()