from datetime import datetime
import logging

logger = logging.getLogger(__name__)


def create_record(record_id):
    logger.info(
        "core_service.create_record.start",
        extra={
            "method": "create_record",
            "step": "validate_input",
            "entity_id": record_id,
        },
    )
    timestamp = datetime.utcnow().isoformat()
    return {
        "id": record_id,
        "created_at": timestamp,
    }
    logger.info(
        "core_service.create_record.done",
        extra={
            "method": "create_record",
            "step": "validate_input",
            "entity_id": record_id,
        },
    )
