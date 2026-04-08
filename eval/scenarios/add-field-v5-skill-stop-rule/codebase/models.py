from django.db import models


class Entity(models.Model):
    display_name = models.CharField(max_length=100)
    unit_number = models.IntegerField()
    check_in = models.DateTimeField()
    check_out = models.DateTimeField()
    status = models.CharField(max_length=20, default="pending")

    class Meta:
        ordering = ["-check_in"]

    def __str__(self):
        return f"{self.display_name} - Unit {self.unit_number}"
