import os

from datetime import datetime
from typing import Iterable, Any
from pathlib import Path

from cassandra.cqlengine import columns, connection
from cassandra.cqlengine.management import sync_table, create_keyspace_simple
from cassandra.cqlengine.models import Model
from cassandra.auth import PlainTextAuthProvider


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
            'id': columns.Text(primary_key=True),
            **cols
        }
    )

    return DynamicModel


models_template = {
    "file_chunks_text": {
        "provider": columns.Text(),
        "chunk_idx": columns.Integer(),
        "text": columns.Text(),
        "name": columns.Text(),
        "created_ts": columns.DateTime(default=datetime.now),
    },
    "file_chunks_embedding": {
        "provider": columns.Text(),
        "chunk_idx": columns.Integer(),
        "embedding": columns.List(value_type=columns.Float),
        "name": columns.Text(),
        "created_ts": columns.DateTime(default=datetime.now),
    },
    "files_full_text": {
        "chunks_cnt": columns.Integer(),
        "text": columns.Text(),
        "name": columns.Text(),
        "created_ts": columns.DateTime(default=datetime.now),
    },
}


def sync_tables(models: Iterable[Any]):
    [sync_table(m) for m in models]


def get_cassandra_session(
        keyspace: str,
        context,
        username: str = 'cassandra',
        password: str = 'cassandra',
) -> Any:
    connection.setup(
        context.c_setup_data['hosts'], keyspace,
        port=context.c_setup_data['port'],
        auth_provider=PlainTextAuthProvider(
            username=username, password=password
        )
    )

    query = f"""
        CREATE KEYSPACE IF NOT EXISTS {keyspace}
        WITH replication = {{ 'class': 'SimpleStrategy', 'replication_factor': '1' }};
    """
    session = connection.get_session()
    session.execute(query)
    session.set_keyspace(keyspace)
    return session


def bootstrap_keyspace(
        keyspace: str,
        workdir: Path,
        context,
) -> None:
    session = get_cassandra_session(keyspace=keyspace, context=context)
    workdir.mkdir(parents=True, exist_ok=True)

    session_cfg = {
        'session': session,
        'workdir': workdir,
        'provider': 'gte'
    }

    for model_name, model_config in models_template.items():
        model_class = create_model_class(keyspace, model_name, model_config)
        session_cfg.setdefault('models', {}).setdefault(model_name, model_class)

    sync_tables(session_cfg['models'].values())
    context.c_sessions.setdefault(keyspace, session_cfg)
