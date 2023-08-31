import json
import uuid

from hashlib import sha1
from datetime import datetime
from typing import Iterable, List

from refact_vecdb.common.profiles import VDBFiles
from refact_vecdb.daemon.context import CONTEXT as C
from refact_vecdb.daemon.params import File2Upload
from refact_vecdb import VDBEmbeddingsAPI


def hash_string(string: str) -> str:
    return sha1(string.encode()).hexdigest()[:12]


def get_all_file_names(keyspace: str) -> Iterable[str]:
    session = C.c_sessions[keyspace]['session']

    for row in session.execute("""
        select name from files_full_text;
    """):
        name = row['name']
        yield name


def delete_files_by_name(names_db_drop: Iterable[str], keyspace: str) -> None:
    session = C.c_sessions[keyspace]['session']

    def bulk_candidates_str(names: Iterable[str]) -> str:
        return "(" + ", ".join(f"'{n}'" for n in names) + ")"

    delete_names_str = bulk_candidates_str(names_db_drop)

    tables = ['file_chunks_embedding', 'file_chunks_text', 'files_full_text']
    tables_drop_ids = {}
    for t in tables:
        for row in session.execute(
                f"""
            select id from {t} where name in {delete_names_str} ALLOW FILTERING;
            """
        ):
            tables_drop_ids.setdefault(t, []).append(row['id'])

    for t, ids in tables_drop_ids.items():
        q = f'delete from {t} where id in {bulk_candidates_str(ids)};'
        session.execute(q)


def create_and_insert_chunks(files: List[File2Upload], keyspace: str) -> None:
    provider = C.c_sessions[keyspace]['provider']
    index_files_state = C.c_sessions[keyspace]['workdir'] / VDBFiles.index_files_state

    def write_index_state(file_n: int, total: int):
        print(f'writing index state: {file_n}/{total}')
        with index_files_state.open('w') as f:
            f.write(json.dumps({
                'file_n': file_n,
                'total': total,
            }))

    emb_api = VDBEmbeddingsAPI()

    models = C.c_sessions[keyspace]['models']
    file_chunks_text = models['file_chunks_text']
    file_chunks_embedding = models['file_chunks_embedding']
    files_full_text = models['files_full_text']

    for idx, file in enumerate(files, 1):
        res_idx = 0
        for res_idx, res in enumerate(emb_api.create(
                {'name': file.name, 'text': file.text},
                provider,
                is_index='True'
        ), 1):
            file_chunks_text_mapping = {
                'id': str(uuid.uuid4())[:12],
                'provider': provider,
                "chunk_idx": res['chunk_idx'],
                'text': res['chunk'],
                'name': res['name'],
                'created_ts': datetime.now()
            }

            file_chunks_text.create(**file_chunks_text_mapping)

            file_chunks_embedding_mapping = {
                'id': file_chunks_text_mapping['id'],
                'provider': file_chunks_text_mapping['provider'],
                "chunk_idx": file_chunks_text_mapping['chunk_idx'],
                'embedding': res['embedding'],
                'name': file_chunks_text_mapping['name'],
                'created_ts': file_chunks_text_mapping['created_ts']
            }

            file_chunks_embedding.create(**file_chunks_embedding_mapping)

        files_full_text_mapping = {
            'id': hash_string(file.text),
            'chunks_cnt': res_idx,
            'text': file.text,
            'name': file.name,
            'created_ts': datetime.now()
        }

        files_full_text.create(**files_full_text_mapping)
        write_index_state(idx, len(files))


def insert_files(files: Iterable[File2Upload], keyspace: str) -> None:
    session = C.c_sessions[keyspace]['session']

    files = list(files)
    file_names = {f.name for f in files}

    names_db_drop = set()
    names_rejected = set()
    for row in session.execute(
            """
            select id, name from files_full_text;
            """):
        idx = row['id']
        name = row['name']

        if name in file_names:
            file = [f for f in files if f.name == name][0]
            if hash_string(file.text) == idx:
                names_rejected.add(name)
                continue
            names_db_drop.add(name)

    if names_db_drop:
        delete_files_by_name(names_db_drop, keyspace)

    files_init_len = files.__len__()
    files = [f for f in files if f.name not in names_rejected]
    print(f'Files passed dup check: {files.__len__()}/{files_init_len}')
    print(f'Names to drop cnt: {names_db_drop.__len__()}')
    if not files:
        return

    create_and_insert_chunks(files, keyspace)


def on_model_change_update_embeddings(keyspace: str) -> None:
    files = []
    session = C.c_sessions[keyspace]['session']

    for row in session.execute(
        """
        select name, text from files_full_text;
        """
    ):
        files.append(File2Upload(name=row['name'], text=row['text']))
    if not files:
        return
    # TODO: create temp table while inserting to not interrupt search
    session.execute('TRUNCATE file_chunks_embedding;')
    session.execute('TRUNCATE file_chunks_text;')
    session.execute('TRUNCATE files_full_text;')

    create_and_insert_chunks(files, keyspace)
