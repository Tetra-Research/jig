from django.urls import path

from . import views

urlpatterns = [
    path("api/entities/<int:pk>/", views.entity_detail, name="entity-detail"),
]
