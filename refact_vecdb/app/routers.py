import json
import itertools

from typing import Dict, Iterable, List
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
from code_extensions import code_extensions_compact


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

        files_cnt = C.c_session.execute('SELECT COUNT(*) FROM files_description;').one()['count']
        chunks_cnt = C.c_session.execute('SELECT COUNT(*) FROM code_files;').one()['count']

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
        FileUpload = namedtuple('FileUpload', ['name', 'text'])

        def files_filter(files: Iterable[FileUpload]) -> List[FileUpload]:
            files = [
                f for f in files
                if Path(f.name).suffix[1:] in code_extensions_compact and f.text
            ]
            return files

        x_token = request.headers.get('X-Auth-Token')
        final_batch: bool = data.final

        files = files_filter([FileUpload(*f) for f in data.files])


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
