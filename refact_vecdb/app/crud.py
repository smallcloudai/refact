from datetime import datetime

from hashlib import sha1
from typing import List

from refact_vecdb.app.context import CONTEXT as C
from refact_vecdb.app.db_models import FileChunksText, FileChunksEmbedding, FilesFullText
from refact_vecdb.app.encoder import ChunkifyFiles


def hash_string(string: str) -> str:
    return sha1(string.encode()).hexdigest()[:12]


def insert_files(
        files: List
):
    file_names = {f.name for f in files}
    print(f'file_names: {file_names}')

    names_db_drop = set()
    names_rejected = set()
    for row in C.c_session.execute(
            """
            select id, name from files_full_text;
            """):
        idx = row['id']
        name = row['name']
        print(f'idx: {idx}, name: {name}')

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

    ch_files = ChunkifyFiles(window_size=512, soft_limit=512)

    mappings = [
        {
            'id': hash_string(chunk),
            'text': chunk,
            'name': file.name,
            'created_ts': datetime.now()
        }
        for file in files for chunk in ch_files.chunkify(file.text)
    ]
    mappings = [m for m in mappings]

    if mappings:
        for m in mappings:
            FileChunksText.create(**m)

        embed_mappings = [
            {
                'id': m['id'],
                'embedding': C.Encoder.encode(m['text']),
                'name': m['name'],
                'created_ts': datetime.now()
            }
            for m in mappings
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

    return len(files)
