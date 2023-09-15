import time
import json
import uuid

from hashlib import sha1
from datetime import datetime
from typing import Iterable, List, Dict

from refact_vecdb.common.profiles import VDBFiles, PROFILES as P
from refact_vecdb.common.context import CONTEXT as C
from refact_vecdb.common.crud import get_account_data
from refact_vecdb.daemon.params import File2Upload
from refact_vecdb import VDBEmbeddingsAPI


def hash_string(string: str) -> str:
    return sha1(string.encode()).hexdigest()[:12]


def get_all_file_names(account: str) -> Iterable[str]:
    session = C.c_session
    for row in session.execute(session.prepare('select name from files_full_text where account =?;'), [account]):
        yield row['name']


def bulk_candidates_str(names: Iterable[str]) -> str:
    return "(" + ", ".join(f"'{n}'" for n in names) + ")"


def get_all_active_embeddings(account: str) -> Iterable[Dict]:
    session = C.c_session

    for row in session.execute(
        f"""
        select id, embedding from file_chunks_embedding where account = '{account}' and active = True ALLOW FILTERING;
        """
    ):
        yield row


def delete_files_by_name(names_db_drop: Iterable[str], account: str) -> None:
    session = C.c_session

    delete_names_str = bulk_candidates_str(names_db_drop)

    tables = ['file_chunks_embedding', 'file_chunks_text', 'files_full_text']
    tables_drop_ids = {}
    for t in tables:
        for row in session.execute(
            f"""
            select id from {t} where name in {delete_names_str} and account = '{account}' ALLOW FILTERING;
            """
        ):
            tables_drop_ids.setdefault(t, []).append(row['id'])

    for t, ids in tables_drop_ids.items():
        q = f"delete from {t} where id in {bulk_candidates_str(ids)} and account='{account}';"
        session.execute(q)


def change_files_active_by_name(names: Iterable[str], account: str, active: bool) -> None:
    session = C.c_session

    alter_names_str = bulk_candidates_str(names)
    tables = ['file_chunks_embedding', 'file_chunks_text', 'files_full_text']
    tables_alter_ids = {}
    for t in tables:
        for row in session.execute(
            f"""
            select id from {t} where name in {alter_names_str} and account = '{account}' ALLOW FILTERING;
            """
        ):
            tables_alter_ids.setdefault(t, []).append(row['id'])

    for t, ids in tables_alter_ids.items():
        q = f"update {t} set active = {active} where id in {bulk_candidates_str(ids)} and account='{account}';"
        session.execute(q)


def set_all_files_active(account: str) -> None:
    session = C.c_session
    tables = ['file_chunks_embedding', 'file_chunks_text', 'files_full_text']

    tables_mod_id = {}
    for t in tables:
        for row in session.execute(f"select id, active from {t} where account = '{account}';"):
            if not row['active']:
                tables_mod_id.setdefault(t, []).append(row['id'])

    for t, ids in tables_mod_id.items():
        q = f"update {t} set active = True where id in {bulk_candidates_str(ids)} and account='{account}';"
        session.execute(q)


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


def create_and_insert_chunks(files: List[File2Upload], account: str) -> None:
    provider = get_account_data(account).get('provider', 'gte')
    index_files_state = P[account]['workdir'] / VDBFiles.index_files_state

    def write_index_state(file_n: int, total: int):
        print(f'writing index state: {file_n}/{total}')
        with index_files_state.open('w') as f:
            f.write(json.dumps({
                'file_n': file_n,
                'total': total,
            }))

    emb_api = VDBEmbeddingsAPI()

    models = C.c_models
    file_chunks_text = models['file_chunks_text']
    file_chunks_embedding = models['file_chunks_embedding']
    files_full_text = models['files_full_text']

    for idx, file in enumerate(files, 1):
        res_idx = 0
        for res_idx, res in enumerate(emb_api.create(
                {'name': file.name, 'text': file.text},
                provider,
                is_index=True
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

            retry_mech(file_chunks_text.create, **file_chunks_text_mapping)

            file_chunks_embedding_mapping = {
                'id': file_chunks_text_mapping['id'],
                "account": account,
                'provider': file_chunks_text_mapping['provider'],
                "chunk_idx": file_chunks_text_mapping['chunk_idx'],
                'embedding': res['embedding'],
                'name': file_chunks_text_mapping['name'],
                'created_ts': file_chunks_text_mapping['created_ts']
            }

            retry_mech(file_chunks_embedding.create, **file_chunks_embedding_mapping)

        files_full_text_mapping = {
            'id': hash_string(file.text),
            "account": account,
            'chunks_cnt': res_idx,
            'text': file.text,
            'name': file.name,
            'created_ts': datetime.now()
        }

        retry_mech(files_full_text.create, **files_full_text_mapping)
        write_index_state(idx, len(files))


def insert_files(files: Iterable[File2Upload], account: str) -> None:
    session = C.c_session

    files = list(files)
    file_names = {f.name for f in files}

    names_db_drop = set()
    names_rejected = set()
    for row in session.execute(
            session.prepare('select id, name from files_full_text where account =?;'),
            [account]
    ):
        idx = row['id']
        name = row['name']

        if name in file_names:
            file = [f for f in files if f.name == name][0]
            if hash_string(file.text) == idx:
                names_rejected.add(name)
                continue
            names_db_drop.add(name)

    if names_db_drop:
        delete_files_by_name(names_db_drop, account)

    files_init_len = files.__len__()
    files = [f for f in files if f.name not in names_rejected]
    print(f'Files passed dup check: {files.__len__()}/{files_init_len}')
    print(f'Names to drop cnt: {names_db_drop.__len__()}')
    if not files:
        return

    create_and_insert_chunks(files, account)


def on_model_change_update_embeddings(account: str) -> None:
    files = []
    session = C.c_session

    for row in session.execute(
            session.prepare('select name, text from files_full_text where account =?;'),
            [account]
    ):
        files.append(File2Upload(name=row['name'], text=row['text']))
    if not files:
        return

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

    create_and_insert_chunks(files, account)
