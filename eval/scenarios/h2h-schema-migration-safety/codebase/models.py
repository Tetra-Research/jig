from django.db import models


class Entity(models.Model):
    name = models.CharField(max_length=255)
