import pytest
from services.booking import BookingService


class TestBookingService:
    """Tests for BookingService."""

    def setup_method(self):
        self.service = BookingService()

    def test_create_booking(self):
        result = self.service.create_booking(
            guest_name="Alice",
            check_in="2024-01-01",
            check_out="2024-01-03",
        )
        assert result is not None

    def test_cancel_booking(self):
        result = self.service.cancel_booking(booking_id="123")
        assert result["status"] == "cancelled"
