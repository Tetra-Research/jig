from django.urls import path
from . import views

urlpatterns = [
    path("api/entities/", views.entity_list, name="entity-list"),
    path("api/entities/<int:pk>/", views.entity_detail, name="entity-detail"),
]
