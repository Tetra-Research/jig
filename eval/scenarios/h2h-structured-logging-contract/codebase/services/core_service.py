from datetime import datetime


def create_record(record_id):
    timestamp = datetime.utcnow().isoformat()
    return {
        "id": record_id,
        "created_at": timestamp,
    }
