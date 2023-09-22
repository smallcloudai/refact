import os
import time
import json
import uuid

from hashlib import sha1
from pathlib import Path
from datetime import datetime
from typing import Iterable, List, Dict, Optional, Any

from more_itertools import chunked

from refact_vecdb.common.context import VDBFiles, CONTEXT as C, upd_file_stats
from refact_vecdb.common.crud import get_account_data
from refact_vecdb import VDBEmbeddingsAPI


class RaiseIfChanged:
    def __init__(self):
        self._data = {}

    def add(self, file: Path, exc: Any):
        if not file.exists():
            print(f'RaiseIfChanged: File {file} does not exist')
            return
        last_mod = os.path.getmtime(file)
        self._data[file] = {'last_mod': last_mod, 'exc': exc}

    def __call__(self):
        for k, v in self._data.items():
            if v['last_mod'] != os.path.getmtime(k):
                raise v['exc']


class DBSetChangedException(Exception):
    pass


class ConfigChangedException(Exception):
    pass


def hash_string(string: str) -> str:
    return sha1(string.encode()).hexdigest()[:12]


def read_and_compare_files(account: str):
    def retrieve_all_file_names() -> Iterable[str]:
        for row in C.c_session.execute(
                C.c_session.prepare('select name from files_full_text where account =?;'),
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
    if diff_file_names := file_names_db - set(str(p) for p in file_paths):
        delete_files_by_name(diff_file_names, account)

    print(f'Found {len(file_paths)} files to insert')

    if not file_paths:
        return

    read_files_gen = ({'path': str(p), 'text': p.read_text()} for p in file_paths)
    insert_files(read_files_gen, account)


def delete_files_by_name(names_db_drop: Iterable[str], account: str) -> None:
    session = C.c_session

    tables = ['file_chunks_embedding', 'file_chunks_text', 'files_full_text']
    for t in tables:
        for d_name in names_db_drop:
            print(f'DROPPING {t} with name {d_name}')
            for row in session.execute(
                f"""
                select id from {t} where name = '{d_name}' and account = '{account}' ALLOW FILTERING;
                """
            ):
                session.execute(f"delete from {t} where id = '{row['id']}' and account = '{account}';")


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


def create_and_insert_chunks(files: List[Dict], account: str, provider: Optional[str] = None) -> None:
    r = RaiseIfChanged()
    r.add(VDBFiles.database_set, DBSetChangedException)
    r.add(VDBFiles.config, ConfigChangedException)

    files_cnt = len(files)
    provider = provider or get_account_data(account).get('provider', 'gte')
    emb_api = VDBEmbeddingsAPI()

    models = C.c_models
    file_chunks_text_tbl = models['file_chunks_text']
    file_chunks_embedding_tbl = models['file_chunks_embedding']
    files_full_text_tbl = models['files_full_text']

    for file_idx, file in enumerate(files, 1):
        res_idx = 0
        r()
        # request to embeds api
        print(f'FILE: {file["path"]}')
        for res_idx, res in enumerate(emb_api.create(
                {'name': file['path'], 'text': file['text']},
                provider,
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
        upd_file_stats({'file_n': file_idx, 'total': files_cnt})


def insert_files(files: Iterable[Dict[str, str]], account: str) -> None:
    session = C.c_session

    def retrieve_files_id():
        for row in session.execute(
                session.prepare('select id from files_full_text where account =?;'),
                [account]
        ):
            yield row['id']

    ids_present = set(retrieve_files_id())
    batch_size = 1_000
    for ch_idx, files_batch in enumerate(chunked(files, batch_size)):
        print(f'inserting batch {ch_idx}')
        files_batch_dict = {hash_string(f['text']): f for f in files_batch}
        files_batch_ids = set(files_batch_dict.keys())
        if dups := ids_present.intersection(files_batch_ids):
            files_batch_dict = {k: v for k, v in files_batch_dict.items() if k not in dups}
        if not files_batch_dict:
            continue
        files_batch_dicts = [{'id': k, **v} for k, v in files_batch_dict.items()]
        create_and_insert_chunks(files_batch_dicts, account)


def on_model_change_update_embeddings(account: str, provider: str) -> None:
    r = RaiseIfChanged()
    r.add(VDBFiles.database_set, DBSetChangedException)
    r.add(VDBFiles.config, ConfigChangedException)
    time.sleep(5)
    r()

    session = C.c_session

    def delete_from_file_chunks_embedding(account: str) -> None:
        ids = session.execute(session.prepare('select id, account from file_chunks_embedding where account =?;'), [account])
        for row in ids:
            session.execute(session.prepare('delete from file_chunks_embedding where id =? and account = ?;'), [row['id'], row['account']])

    def delete_from_file_chunks_text(account: str) -> None:
        ids = session.execute(session.prepare('select id, account from file_chunks_text where account =?;'), [account])
        for row in ids:
            session.execute(session.prepare('delete from file_chunks_text where id =? and account =?;'), [row['id'], row['account']])

    def delete_from_files_full_text(account: str) -> None:
        ids = session.execute(session.prepare('select id, account from files_full_text where account =?;'), [account])
        for row in ids:
            session.execute(session.prepare('delete from files_full_text where id =? and account =?;'), [row['id'], row['account']])

    delete_from_file_chunks_embedding(account)
    delete_from_file_chunks_text(account)
    delete_from_files_full_text(account)

    def retrieve_files_name_text():
        for row in session.execute(
                session.prepare('select name, text from files_full_text where account =?;'),
                [account]
        ):
            yield {'path': row['name'], 'text': row['text']}

    batch_size = 1_000
    for files_batch in chunked(retrieve_files_name_text(), batch_size):
        create_and_insert_chunks(files_batch, account, provider)
