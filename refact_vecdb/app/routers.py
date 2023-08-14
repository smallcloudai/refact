import json
import itertools

from typing import Dict, Iterable, List
from pathlib import Path
from collections import namedtuple

from fastapi import APIRouter
from fastapi import Response, Request

from refact_vecdb.app.context import CONTEXT as C
from refact_vecdb.app.db_models import FileChunksText, FileChunksEmbedding, FilesFullText
from refact_vecdb.app.params import FindQuery, FilesBulk
from refact_vecdb.app.bootstrap import load_vecdb
from refact_vecdb.app.encoder import ChunkifyFiles
from refact_vecdb.app.crud import insert_files


class StatusRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(StatusRouter, self).__init__(*args, **kwargs)
        super(StatusRouter, self).add_api_route("/v1/status", self._status, methods=["GET"])
        super(StatusRouter, self).add_api_route("/v1/health", self._health, methods=["GET"])
        super(StatusRouter, self).add_api_route("/v1/files-stats", self._files_stats, methods=["GET"])

    async def _status(self, request: Request):
        x_token = request.headers.get('X-Auth-Token')

        return Response(content=json.dumps({
            "status": "ok"
        }))

    async def _health(self, request: Request):
        return Response(content=json.dumps({
            "status": "ok"
        }))

    async def _files_stats(self, request: Request):
        x_token = request.headers.get('X-Auth-Token')

        files_cnt = C.c_session.execute('SELECT COUNT(*) FROM files_full_text;').one()['count']
        chunks_cnt = C.c_session.execute('SELECT COUNT(*) FROM file_chunks_text;').one()['count']

        return Response(content=json.dumps(
            {"files_cnt": files_cnt, "chunks_cnt": chunks_cnt}
        ))


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
        query: Dict[str, FileChunksText] = {
            q.id: q for q in FileChunksText.filter(id__in=list(set(itertools.chain(*ids))))
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
        FileUpload = namedtuple('FileUpload', ['name', 'text'])
        print('uploading files...')

        x_token = request.headers.get('X-Auth-Token')
        final_batch: bool = data.final

        files = [FileUpload(*f) for f in data.files]

        inserted_f_cnt = insert_files(files)

        C.vecdb_update_required = True
        if final_batch and inserted_f_cnt:
            load_vecdb()
        return Response(status_code=200)


class DeleteAllRecordsRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(DeleteAllRecordsRouter, self).__init__(*args, **kwargs)
        super(DeleteAllRecordsRouter, self).add_api_route("/v1/delete_all", self._delete_all, methods=["GET"])

    async def _delete_all(self, request: Request):
        x_token = request.headers.get('X-Auth-Token')

        C.c_session.execute('TRUNCATE file_chunks_embedding;')
        C.c_session.execute('TRUNCATE file_chunks_text;')
        C.c_session.execute('TRUNCATE files_full_text;')
        return Response(status_code=200)
