from datetime import datetime


class CoreService:
    """Handles entity record logic."""

    def create_record(self, display_name: str, check_in: datetime, check_out: datetime) -> dict:
        if check_out <= check_in:
            raise ValueError("Check-out must be after check-in")
        return {
            "display_name": display_name,
            "check_in": check_in.isoformat(),
            "check_out": check_out.isoformat(),
            "status": "confirmed",
        }

    def cancel_record(self, record_id: str) -> dict:
        return {"record_id": record_id, "status": "cancelled"}
