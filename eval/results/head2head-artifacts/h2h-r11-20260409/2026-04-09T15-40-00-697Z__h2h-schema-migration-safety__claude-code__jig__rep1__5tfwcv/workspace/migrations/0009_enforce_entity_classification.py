from django.db import migrations, models


class Migration(migrations.Migration):
    dependencies = [
        ("core", "0008_add_entity_classification"),
    ]

    operations = [
        migrations.AlterField(
            model_name="entity",
            name="classification",
            field=models.CharField(max_length=20),
        ),
    ]