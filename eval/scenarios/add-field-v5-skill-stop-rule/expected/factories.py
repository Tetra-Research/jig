import factory
from .models import Reservation


class ReservationFactory(factory.django.DjangoModelFactory):
    class Meta:
        model = Reservation

    guest_name = factory.Faker("name")
    room_number = factory.Sequence(lambda n: 100 + n)
    check_in = factory.Faker("date_time_this_year")
    check_out = factory.Faker("date_time_this_year")
    status = "confirmed"
    loyalty_tier = "bronze"
