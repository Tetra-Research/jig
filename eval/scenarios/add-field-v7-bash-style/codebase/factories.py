import factory
from .models import Entity


class EntityFactory(factory.django.DjangoModelFactory):
    class Meta:
        model = Entity

    display_name = factory.Faker("name")
    unit_number = factory.Sequence(lambda n: 100 + n)
    check_in = factory.Faker("date_time_this_year")
    check_out = factory.Faker("date_time_this_year")
    status = "confirmed"
