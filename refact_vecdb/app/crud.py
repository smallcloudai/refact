from datetime import datetime

from hashlib import sha1
from typing import List

from context import CONTEXT as C
from db_models import CodeFiles, FilesEmbedding, FilesDescription
from encoder import ChunkifyFiles


def hash_string(string: str) -> str:
    return sha1(string.encode()).hexdigest()[:12]


def insert_files(files: List):
    ch_files = ChunkifyFiles(window_size=512, soft_limit=512)

    code_files_ids_ex = set()
    for row in C.c_session.execute(
            """
            select id from code_files;
            """):
        code_files_ids_ex.add(row['id'])
    mappings = [
        {
            'id': hash_string(chunk),
            'text': chunk,
            'name': file.name,
            'created_ts': datetime.now()
        }
        for file in files for chunk in ch_files.chunkify(file.text)
    ]
    init_len = len(mappings)
    mappings = [m for m in mappings if m['id'] not in code_files_ids_ex]
    print(f'SKIPPED {init_len - len(mappings)} files as replicates [mappings]')
    if mappings:
        for m in mappings:
            CodeFiles.create(**m)

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
            FilesEmbedding.create(**m)

    files_desc_ids_ex = set()
    for row in C.c_session.execute(
            """
            select id from files_description;
            """):
        files_desc_ids_ex.add(row['id'])
    files_desc_mappings = [
        {
            'id': hash_string(file.text),
            'text': file.text,
            'name': file.name,
            'created_ts': datetime.now()
        }
        for file in files
    ]
    init_len = len(files_desc_mappings)
    files_desc_mappings = [m for m in files_desc_mappings if m['id'] not in files_desc_ids_ex]
    print(f'SKIPPED {init_len - len(files_desc_mappings)} files as replicates [files_desc_mappings]')
    for m in files_desc_mappings:
        FilesDescription.create(**m)
