from django.db import models




class EntityQuerySet(models.QuerySet):
    def active(self):
        return self.filter(status="active")


class EntityManager(models.Manager):
    def get_queryset(self):
        return EntityQuerySet(self.model, using=self._db)

    def active(self):
        return self.get_queryset().active()
class Entity(models.Model):
    objects = EntityManager()
    status = models.CharField(max_length=20)
    display_name = models.CharField(max_length=255)
