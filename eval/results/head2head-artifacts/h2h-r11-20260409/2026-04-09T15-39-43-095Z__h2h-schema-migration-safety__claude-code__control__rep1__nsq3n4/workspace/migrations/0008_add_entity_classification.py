from django.db import migrations, models


def backfill_classification(apps, schema_editor):
    model_cls = apps.get_model("core", "Entity")
    db_alias = schema_editor.connection.alias
    for row in model_cls.objects.using(db_alias).all().only("id", "classification"):
        if row.classification is None or row.classification == "":
            row.classification = "standard"
            row.save(update_fields=["classification"])


class Migration(migrations.Migration):
    dependencies = [
        ("core", "0007_auto_20260407_1200"),
    ]

    operations = [
        migrations.AddField(
            model_name="Entity",
            name="classification",
            field=models.CharField(max_length=20, null=True),
        ),
        migrations.RunPython(backfill_classification, migrations.RunPython.noop),
    ]
