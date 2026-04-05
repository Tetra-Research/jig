from django.db import models
from django.db.models import CharField, IntegerField
from django.db.models import DateTimeField


class Reservation(models.Model):
    guest_name = CharField(max_length=100)
    room_number = IntegerField()
    check_in = models.DateTimeField()
    check_out = models.DateTimeField()
    status = CharField(max_length=20, default="pending")

    class Meta:
        ordering = ["-check_in"]
