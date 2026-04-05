from django.contrib import admin
from .models import Reservation


@admin.register(Reservation)
class ReservationAdmin(admin.ModelAdmin):
    list_display = [
        "guest_name",
        "room_number",
        "check_in",
        "check_out",
        "status",
    ]
    list_filter = ["status", "check_in"]
    search_fields = ["guest_name"]
