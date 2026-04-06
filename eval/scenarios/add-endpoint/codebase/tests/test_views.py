from django.test import TestCase, Client


class TestReservationViews(TestCase):
    def setUp(self):
        self.client = Client()

    def test_reservation_list(self):
        response = self.client.get("/api/reservations/")
        self.assertEqual(response.status_code, 200)

    def test_reservation_detail(self):
        response = self.client.get("/api/reservations/1/")
        self.assertIn(response.status_code, [200, 404])
