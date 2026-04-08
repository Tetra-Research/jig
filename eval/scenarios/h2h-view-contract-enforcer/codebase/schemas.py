from rest_framework import serializers


class EntityDetailResponse(serializers.Serializer):
    id = serializers.IntegerField()
