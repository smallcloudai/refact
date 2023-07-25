import json
import itertools

from typing import Dict
from pathlib import Path
from collections import namedtuple

from fastapi import APIRouter
from fastapi import Response, Request

from context import CONTEXT as C
from db_models import CodeFiles, FilesEmbedding, FilesDescription
from params import FindQuery, FilesBulk
from bootstrap import load_vecdb
from encoder import ChunkifyFiles
from crud import insert_files


class FindRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(FindRouter, self).__init__(*args, **kwargs)
        super(FindRouter, self).add_api_route("/v1/find", self._find, methods=["POST"])

    async def _find(self, data: FindQuery, request: Request):
        x_token = request.headers.get('X-Auth-Token')

        if C.vecdb_update_required:
            load_vecdb()

        ch_files = ChunkifyFiles(window_size=512, soft_limit=512)
        chunks = [chunk for chunk in ch_files.chunkify(data.query)]
        embeddings = [C.Encoder.encode(c) for c in chunks]
        ids, scores = C.db.search(embeddings, data.top_k)
        query: Dict[str, CodeFiles] = {
            q.id: q for q in CodeFiles.filter(id__in=list(set(itertools.chain(*ids))))
        }
        result = [
            {
                'file_path': query[uid].name,
                'file_name': Path(query[uid].name).name,
                'text': query[uid].text,
                # 'score': round(score, 3)
            }
            for uid_batch, scores_batch in zip(ids, scores)
            for uid, score in zip(uid_batch, scores_batch)
        ]

        return Response(content=json.dumps({
            "results": result
        }))


class UploadRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(UploadRouter, self).__init__(*args, **kwargs)
        super(UploadRouter, self).add_api_route("/v1/bulk_upload", self._bulk_upload, methods=["POST"])

    async def _bulk_upload(self, data: FilesBulk, request: Request):
        x_token = request.headers.get('X-Auth-Token')
        final_batch: bool = data.final

        FileUpload = namedtuple('FileUpload', ['name', 'text'])
        files = [FileUpload(*f) for f in data.files]
        insert_files(files)

        C.vecdb_update_required = True
        if final_batch:
            load_vecdb()
        return Response(status_code=200)


class DeleteAllRecordsRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(DeleteAllRecordsRouter, self).__init__(*args, **kwargs)
        super(DeleteAllRecordsRouter, self).add_api_route("/v1/delete_all", self._delete_all, methods=["POST"])

    async def _delete_all(self, request: Request):
        x_token = request.headers.get('X-Auth-Token')

        C.c_session.execute('DELETE FROM files_embedding;')
        C.c_session.execute('DELETE FROM code_files;')
        C.c_session.commit()
        return Response(status_code=200)
