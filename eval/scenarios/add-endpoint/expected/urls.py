from django.urls import path
from . import views

urlpatterns = [
    path("api/reservations/", views.reservation_list, name="reservation-list"),
    path("api/reservations/<int:pk>/", views.reservation_detail, name="reservation-detail"),
    path("api/reservations/<int:pk>/receipt/", views.reservation_receipt, name="reservation-receipt"),
]
