from django.test import TestCase, Client


class TestEntityViews(TestCase):
    def setUp(self):
        self.client = Client()

    def test_entity_list(self):
        response = self.client.get("/api/entities/")
        self.assertEqual(response.status_code, 200)

    def test_entity_detail(self):
        response = self.client.get("/api/entities/1/")
        self.assertIn(response.status_code, [200, 404])
