import time
import json
import uuid

from hashlib import sha1
from datetime import datetime
from typing import Iterable, List, Dict

from more_itertools import chunked

from refact_vecdb.common.context import VDBFiles, CONTEXT as C
from refact_vecdb import VDBEmbeddingsAPI


def hash_string(string: str) -> str:
    return sha1(string.encode()).hexdigest()[:12]


def erase_files_by_name(cfg_tracker, names_to_drop: Iterable[str], account: str) -> None:
    session = C.c_session

    tables = ['file_chunks_embedding', 'file_chunks_text', 'files_full_text']
    for d_name in names_to_drop:
        cfg_tracker.throw_if_changed()
        for t in tables:
            for row in session.execute(
                f"""
                select id from {t} where name = '{d_name}' and account = '{account}' ALLOW FILTERING;
                """
            ):
                session.execute(f"delete from {t} where id = '{row['id']}' and account = '{account}';")


def read_and_compare_files(cfg_tracker, provider: str):
    account = cfg_tracker.account

    def retrieve_all_file_names() -> Iterable[str]:
        for row in C.c_session.execute(
                C.c_session.prepare('SELECT name FROM files_full_text WHERE account =?;'),
                [account]
        ):
            yield row['name']

    file_paths = []
    for line_raw in VDBFiles.database_set.read_text().splitlines():
        if not line_raw:
            continue
        line = json.loads(line_raw)
        file_paths.append(VDBFiles.workdir / line['path'])

    # if file exists in DB but does not exist in database_set_file -> delete it from DB
    file_names_db = set(retrieve_all_file_names())
    if to_delete := file_names_db - set(str(p) for p in file_paths):
        erase_files_by_name(cfg_tracker, to_delete, account)

    if not file_paths:
        return

    add_or_update_generator = ({'path': str(p), 'text': p.read_text()} for p in file_paths)
    insert_files(cfg_tracker, add_or_update_generator, len(file_paths), provider)


def retry_mech(func, *args, **kwargs):
    tries = 3
    while True:
        try:
            return func(*args, **kwargs)
        except Exception as e:
            print(f'Retrying {func.__name__}; tries left: {tries}')
            time.sleep(0.5)
            tries -= 1
            if tries == 0:
                raise e


def create_and_insert_chunks(
        cfg_tracker,
        files: List[Dict],
        from_file_n: int,
        files_total: int,
        provider: str,
        throw_if_changed: bool = True
) -> None:
    account: str = cfg_tracker.account
    emb_api = VDBEmbeddingsAPI()

    models = C.c_models
    file_chunks_text_tbl = models['file_chunks_text']
    file_chunks_embedding_tbl = models['file_chunks_embedding']
    files_full_text_tbl = models['files_full_text']

    for file_idx, file in enumerate(files, 1):
        if throw_if_changed:
            cfg_tracker.throw_if_changed()
        cfg_tracker.upd_stats(from_file_n + file_idx, files_total)
        res_idx = 0
        # request to embeds api
        for res_idx, res in enumerate(emb_api.create(
                texts={'name': file['path'], 'text': file['text']},
                provider=provider,
        ), 1):
            file_chunks_text_mapping = {
                'id': str(uuid.uuid4())[:12],
                "account": account,
                'provider': provider,
                "chunk_idx": res['chunk_idx'],
                'text': res['chunk'],
                'name': res['name'],
                'created_ts': datetime.now()
            }

            retry_mech(file_chunks_text_tbl.create, **file_chunks_text_mapping)

            file_chunks_embedding_mapping = {
                'id': file_chunks_text_mapping['id'],
                "account": account,
                'provider': file_chunks_text_mapping['provider'],
                "chunk_idx": file_chunks_text_mapping['chunk_idx'],
                'embedding': res['embedding'],
                'name': file_chunks_text_mapping['name'],
                'created_ts': file_chunks_text_mapping['created_ts']
            }

            retry_mech(file_chunks_embedding_tbl.create, **file_chunks_embedding_mapping)

        files_full_text_mapping = {
            'id': file['id'],
            "account": account,
            'chunks_cnt': res_idx,
            'text': file['text'],
            'name': file['path'],
            'created_ts': datetime.now()
        }

        retry_mech(files_full_text_tbl.create, **files_full_text_mapping)


def insert_files(
        cfg_tracker,
        files_generator: Iterable[Dict[str, str]],
        files_total: int,
        provider: str
) -> None:
    session = C.c_session
    account = cfg_tracker.account

    def retrieve_files_id():
        for row in session.execute(
                session.prepare('select id from files_full_text where account =?;'),
                [account]
        ):
            yield row['id']

    ids_present = set(retrieve_files_id())
    batch_size = 10
    for chunk_n, files_batch in enumerate(chunked(files_generator, batch_size)):
        from_file_n = chunk_n * batch_size
        hash2filedict = {hash_string(f['text']): f for f in files_batch}
        files_batch_ids = set(hash2filedict.keys())
        if dups := ids_present.intersection(files_batch_ids):
            hash2filedict = {k: v for k, v in hash2filedict.items() if k not in dups}
        if not hash2filedict:
            continue
        unique_files = [{'id': k, **v} for k, v in hash2filedict.items()]
        create_and_insert_chunks(cfg_tracker, unique_files, from_file_n, files_total, provider)


def on_model_change_update_embeddings(cfg_tracker, provider: str) -> None:
    session = C.c_session
    account = cfg_tracker.account

    def delete_from_file_chunks_embedding() -> None:
        ids = session.execute(session.prepare('select id, account from file_chunks_embedding where account =?;'), [account])
        for row in ids:
            session.execute(session.prepare('delete from file_chunks_embedding where id =? and account = ?;'), [row['id'], row['account']])

    def delete_from_file_chunks_text() -> None:
        ids = session.execute(session.prepare('select id, account from file_chunks_text where account =?;'), [account])
        for row in ids:
            session.execute(session.prepare('delete from file_chunks_text where id =? and account =?;'), [row['id'], row['account']])

    delete_from_file_chunks_embedding()
    delete_from_file_chunks_text()

    def retrieve_files_name_text() -> Iterable[Dict[str, str]]:
        for row in session.execute(
                session.prepare('select name, text from files_full_text where account =?;'),
                [account]
        ):
            yield {'path': row['name'], 'text': row['text']}

    files_total = session.execute(
        session.prepare('select count(*) from files_full_text where account = ?'),
        [account]
    ).one()['count']

    batch_size = 10
    for chunk_n, files_batch in enumerate(chunked(retrieve_files_name_text(), batch_size)):
        from_file_n = chunk_n * batch_size
        create_and_insert_chunks(cfg_tracker, files_batch, from_file_n, files_total, provider, throw_if_changed=False)
