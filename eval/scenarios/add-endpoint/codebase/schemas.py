from rest_framework import serializers


class ReservationListSchema(serializers.Serializer):
    id = serializers.IntegerField()
    guest = serializers.CharField()


class ReservationDetailSchema(serializers.Serializer):
    id = serializers.IntegerField()
    guest = serializers.CharField()
    room = serializers.IntegerField()
    status = serializers.CharField()
