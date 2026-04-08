import pytest
from services.core_service import CoreService


class TestCoreService:
    """Tests for CoreService."""

    def setup_method(self):
        self.service = CoreService()

    def test_create_record(self):
        result = self.service.create_record(
            display_name="Alice",
            check_in="2024-01-01",
            check_out="2024-01-03",
        )
        assert result is not None

    def test_cancel_record(self):
        result = self.service.cancel_record(record_id="123")
        assert result["status"] == "cancelled"
