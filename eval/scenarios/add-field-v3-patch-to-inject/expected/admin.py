from django.contrib import admin
from .models import Entity


@admin.register(Entity)
class EntityAdmin(admin.ModelAdmin):
    list_display = [
        "classification",
        "display_name",
        "unit_number",
        "check_in",
        "check_out",
        "status",
    ]
    list_filter = ["status", "check_in"]
    search_fields = ["display_name"]
