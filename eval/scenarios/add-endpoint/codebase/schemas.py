from rest_framework import serializers


class EntityListSchema(serializers.Serializer):
    id = serializers.IntegerField()
    display_name = serializers.CharField()


class EntityDetailSchema(serializers.Serializer):
    id = serializers.IntegerField()
    display_name = serializers.CharField()
    unit = serializers.IntegerField()
    status = serializers.CharField()
