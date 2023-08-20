from datetime import datetime

from hashlib import sha1
from math import ceil
from typing import List, Optional, Iterator

from more_itertools import chunked

from refact_vecdb.app.context import CONTEXT as C
from refact_vecdb.app.db_models import FileChunksText, FileChunksEmbedding, FilesFullText
from refact_vecdb.app.params import FileUpload


def hash_string(string: str) -> str:
    return sha1(string.encode()).hexdigest()[:12]


def on_model_change_update_embeddings(batch_size: int) -> Optional[Iterator[int]]:
    files = []
    for row in C.c_session.execute(
        """
        select name, text from files_full_text;
        """
    ):
        files.append(FileUpload(**row))
    if not files:
        return

    C.c_session.execute('TRUNCATE file_chunks_embedding;')
    C.c_session.execute('TRUNCATE files_full_text;')

    total_batches = ceil(len(files) / batch_size)
    for idx, f_batch in enumerate(chunked(files, batch_size), 1):
        create_and_insert_chunks(f_batch)
        yield {'step': str(idx), 'total': str(total_batches)}


def create_and_insert_chunks(files: List[FileUpload]):
    provider = C.provider
    mappings = [
        {
            'id': hash_string(chunk),
            'provider': provider,
            'text': chunk,
            'name': file.name,
            'created_ts': datetime.now()
        }
        for file, file_chunks in
        zip(files, C.encoder.chunkify(f.text for f in files))
        for chunk in file_chunks
    ]
    mappings = [m for m in mappings]

    if mappings:
        for m in mappings:
            FileChunksText.create(**m)

        embeddings = C.encoder.encode([m['text'] for m in mappings])
        embed_mappings = [
            {
                'id': m['id'],
                'provider': m['provider'],
                'embedding': emb,
                'name': m['name'],
                'created_ts': datetime.now()
            }
            for m, emb in zip(mappings, embeddings)
        ]
        for m in embed_mappings:
            FileChunksEmbedding.create(**m)

    files_full_text_mappings = [
        {
            'id': hash_string(file.text),
            'text': file.text,
            'name': file.name,
            'created_ts': datetime.now()
        }
        for file in files
    ]
    files_desc_mappings = [m for m in files_full_text_mappings]
    for m in files_desc_mappings:
        FilesFullText.create(**m)


def insert_files(files: List[FileUpload]) -> int:
    file_names = {f.name for f in files}

    names_db_drop = set()
    names_rejected = set()
    for row in C.c_session.execute(
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

        def bulk_candidates_str(names) -> str:
            return "(" + ", ".join(f"'{n}'" for n in names) + ")"

        delete_names_str = bulk_candidates_str(names_db_drop)

        tables = ['file_chunks_embedding', 'file_chunks_text', 'files_full_text']
        tables_drop_ids = {}
        for t in tables:
            for row in C.c_session.execute(
                f"""
                select id from {t} where name in {delete_names_str} ALLOW FILTERING;
                """
            ):
                tables_drop_ids.setdefault(t, []).append(row['id'])

        for t, ids in tables_drop_ids.items():
            C.c_session.execute(f'delete from {t} where id in {bulk_candidates_str(ids)} ALLOW FILTERING;')

    files_init_len = files.__len__()
    files = [f for f in files if f.name not in names_rejected]
    print(f'Files passed dup check: {files.__len__()}/{files_init_len}')
    print(f'Names to drop cnt: {names_db_drop.__len__()}')
    if not files:
        return 0

    create_and_insert_chunks(files)

    return len(files)


def delete_all_records() -> None:
    C.c_session.execute('TRUNCATE file_chunks_embedding;')
    C.c_session.execute('TRUNCATE file_chunks_text;')
    C.c_session.execute('TRUNCATE files_full_text;')
