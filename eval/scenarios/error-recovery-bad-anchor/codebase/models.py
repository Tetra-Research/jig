from django.db import models
from pydantic import BaseModel as PydanticBase


class Contact(PydanticBase):
    """Contact model using Pydantic base instead of Django Model."""
    first_name: str = ""
    last_name: str = ""
    email: str = ""

    class Config:
        orm_mode = True


class Address(models.Model):
    street = models.CharField(max_length=200)
    city = models.CharField(max_length=100)
    zip_code = models.CharField(max_length=10)
