from rest_framework import serializers
from .models import Entity


class EntitySerializer(serializers.ModelSerializer):
    class Meta:
        model = Entity
        fields = [
            "classification",
            "id",
            "display_name",
            "unit_number",
            "check_in",
            "check_out",
            "status",
        ]
