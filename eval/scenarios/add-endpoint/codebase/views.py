from rest_framework.decorators import api_view
from rest_framework.response import Response
from .models import Reservation


@api_view(["GET"])
def reservation_list(request):
    reservations = Reservation.objects.all()
    return Response([{"id": r.id, "guest": r.guest_name} for r in reservations])


@api_view(["GET"])
def reservation_detail(request, pk):
    reservation = Reservation.objects.get(pk=pk)
    return Response({
        "id": reservation.id,
        "guest": reservation.guest_name,
        "room": reservation.room_number,
        "status": reservation.status,
    })
