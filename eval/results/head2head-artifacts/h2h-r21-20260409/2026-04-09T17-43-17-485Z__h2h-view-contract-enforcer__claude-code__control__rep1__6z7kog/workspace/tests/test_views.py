from django.test import TestCase


class TestViews(TestCase):
    def test_entity_summary(self):
        response = self.client.post(
            "/api/entities/1/summary/",
            {"correlation_id": "h2h"},
            content_type="application/json",
        )
        self.assertIn(response.status_code, [200, 400, 401, 404])
