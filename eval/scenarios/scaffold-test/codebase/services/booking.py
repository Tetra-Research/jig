from datetime import datetime


class BookingService:
    """Handles reservation booking logic."""

    def create_booking(self, guest_name: str, check_in: datetime, check_out: datetime) -> dict:
        if check_out <= check_in:
            raise ValueError("Check-out must be after check-in")
        return {
            "guest": guest_name,
            "check_in": check_in.isoformat(),
            "check_out": check_out.isoformat(),
            "status": "confirmed",
        }

    def cancel_booking(self, booking_id: str) -> dict:
        return {"booking_id": booking_id, "status": "cancelled"}
