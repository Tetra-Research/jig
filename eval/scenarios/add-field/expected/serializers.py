from rest_framework import serializers
from .models import Reservation


class ReservationSerializer(serializers.ModelSerializer):
    class Meta:
        model = Reservation
        fields = [
            "loyalty_tier",
            "id",
            "guest_name",
            "room_number",
            "check_in",
            "check_out",
            "status",
        ]
