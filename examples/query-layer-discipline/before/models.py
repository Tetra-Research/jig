from django.db import models


class Entity(models.Model):
    status = models.CharField(max_length=20)
    display_name = models.CharField(max_length=255)
