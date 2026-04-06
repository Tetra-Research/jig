from django.db import models


class Reservation(models.Model):
    guest_name = models.CharField(max_length=100)
    room_number = models.IntegerField()
    check_in = models.DateTimeField()
    check_out = models.DateTimeField()
    status = models.CharField(max_length=20, default="pending")
    loyalty_tier = models.CharField(max_length=20, default="bronze")

    class Meta:
        ordering = ["-check_in"]

    def __str__(self):
        return f"{self.guest_name} - Room {self.room_number}"
