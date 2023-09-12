import os

from datetime import datetime
from typing import Iterable, Any

from cassandra.cqlengine import columns, connection
from cassandra.cqlengine.management import sync_table
from cassandra.cqlengine.models import Model
from cassandra.auth import PlainTextAuthProvider

from refact_vecdb.common.context import CONTEXT as C

__all__ = ["bootstrap_keyspace"]


os.environ['CQLENG_ALLOW_SCHEMA_MANAGEMENT'] = '1'


class DynamicModelMeta(type):
    def __new__(cls, name, bases, attrs, **kwargs):
        columns_dict = attrs.pop('columns', {})
        new_class = super().__new__(cls, name, bases, attrs, **kwargs)
        for column_name, column in columns_dict.items():
            setattr(new_class, column_name, column)
        return new_class


def create_model_class(keyspace, model_name, cols):
    DynamicModel = type(
        model_name,
        (Model,),
        {
            '__keyspace__': keyspace,
            '__table_name__': model_name,
            **cols
        }
    )

    return DynamicModel


models_template = {
    "file_chunks_text": {
        'id': columns.Text(primary_key=True),
        "account": columns.Text(),
        "provider": columns.Text(),
        "chunk_idx": columns.Integer(),
        "text": columns.Text(),
        "name": columns.Text(),
        "created_ts": columns.DateTime(default=datetime.now),
    },
    "file_chunks_embedding": {
        'id': columns.Text(primary_key=True),
        "account": columns.Text(),
        "provider": columns.Text(),
        "chunk_idx": columns.Integer(),
        "embedding": columns.List(value_type=columns.Float),
        "name": columns.Text(),
        "created_ts": columns.DateTime(default=datetime.now),
    },
    "files_full_text": {
        'id': columns.Text(primary_key=True),
        "account": columns.Text(),
        "chunks_cnt": columns.Integer(),
        "text": columns.Text(),
        "name": columns.Text(),
        "created_ts": columns.DateTime(default=datetime.now),
    },
    "accounts": {
        "account": columns.Text(primary_key=True),
        "team": columns.Text(default=None),
        "provider": columns.Text(default="gte"),
        "created_ts": columns.DateTime(default=datetime.now),
    }
}


def sync_tables(models: Iterable[Any]):
    [sync_table(m) for m in models]


def get_cassandra_session(
        keyspace: str,
        username: str = 'cassandra',
        password: str = 'cassandra',
        replication_strategy: str = 'SimpleStrategy',
        replication_factor: int = 1
) -> Any:
    connection.setup(
        C.c_setup_data['hosts'], keyspace,
        port=C.c_setup_data['port'],
        auth_provider=PlainTextAuthProvider(
            username=username, password=password
        )
    )

    query = f"""
        CREATE KEYSPACE IF NOT EXISTS {keyspace}
        WITH replication = {{ 'class': '{replication_strategy}', 'replication_factor': '{replication_factor}' }};
    """
    session = connection.get_session()
    session.execute(query)
    session.set_keyspace(keyspace)
    return session


def bootstrap_keyspace(
        keyspace: str,
) -> None:
    session = get_cassandra_session(keyspace=keyspace)

    for model_name, model_config in models_template.items():
        model_class = create_model_class(keyspace, model_name, model_config)
        C.c_models.setdefault(model_name, model_class)

    sync_tables(C.c_models.values())

    C.c_session = session
