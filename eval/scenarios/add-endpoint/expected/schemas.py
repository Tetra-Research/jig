from rest_framework import serializers


class ReservationListSchema(serializers.Serializer):
    id = serializers.IntegerField()
    guest = serializers.CharField()


class ReservationDetailSchema(serializers.Serializer):
    id = serializers.IntegerField()
    guest = serializers.CharField()
    room = serializers.IntegerField()
    status = serializers.CharField()


class ReceiptSchema(serializers.Serializer):
    id = serializers.IntegerField()
    guest = serializers.CharField()
    total = serializers.DecimalField(max_digits=10, decimal_places=2)
    currency = serializers.CharField()
