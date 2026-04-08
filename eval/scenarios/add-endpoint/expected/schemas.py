from rest_framework import serializers


class EntityListSchema(serializers.Serializer):
    id = serializers.IntegerField()
    display_name = serializers.CharField()


class EntityDetailSchema(serializers.Serializer):
    id = serializers.IntegerField()
    display_name = serializers.CharField()
    unit = serializers.IntegerField()
    status = serializers.CharField()


class SummarySchema(serializers.Serializer):
    id = serializers.IntegerField()
    display_name = serializers.CharField()
    total = serializers.DecimalField(max_digits=10, decimal_places=2)
    currency = serializers.CharField()
