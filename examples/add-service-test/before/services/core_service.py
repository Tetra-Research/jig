class CoreService:
    def create_record(self, display_name, check_in, check_out):
        return {
            "id": "abc-123",
            "display_name": display_name,
            "status": "confirmed",
        }

    def cancel_record(self, record_id):
        return {
            "id": record_id,
            "status": "cancelled",
        }
