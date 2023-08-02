from datetime import datetime

from cassandra.cqlengine import columns
from cassandra.cqlengine.management import sync_table
from cassandra.cqlengine.models import Model


class FileChunksText(Model):
    id = columns.Text(primary_key=True)
    text = columns.Text()
    name = columns.Text()
    created_ts = columns.DateTime(default=datetime.now)


class FileChunksEmbedding(Model):
    id = columns.Text(primary_key=True)
    embedding = columns.List(value_type=columns.Float)
    name = columns.Text()
    created_ts = columns.DateTime(default=datetime.now)


class FilesFullText(Model):
    id = columns.Text(primary_key=True)
    text = columns.Text()
    name = columns.Text()
    description = columns.Text(default="")
    created_ts = columns.DateTime(default=datetime.now)


def sync_tables():
    for m in [FileChunksText, FileChunksEmbedding, FilesFullText]:
        sync_table(m)
