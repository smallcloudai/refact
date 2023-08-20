import json
import itertools

from typing import Dict, Iterable, List
from pathlib import Path

from fastapi import APIRouter
from fastapi import Response, Request
from fastapi.responses import StreamingResponse

from refact_vecdb.app.context import CONTEXT as C
from refact_vecdb.app.db_models import FileChunksText
from refact_vecdb.app.embed_spads import embed_providers
from refact_vecdb.app.params import FindQuery, FilesBulkUpload, VecDBUpdateProvider, FileUpload
from refact_vecdb.app.bootstrap import load_vecdb
from refact_vecdb.app.crud import insert_files, delete_all_records, on_model_change_update_embeddings


class StatusRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(StatusRouter, self).__init__(*args, **kwargs)
        super(StatusRouter, self).add_api_route("/v1/status", self._status, methods=["GET"])
        super(StatusRouter, self).add_api_route("/v1/files-stats", self._files_stats, methods=["GET"])
        super(StatusRouter, self).add_api_route("/v1/update-provider", self._update_provider, methods=["POST"])

    async def _status(self, request: Request):
        x_token = request.headers.get('X-Auth-Token')
        model = list(C.models.keys()) or ['undefined']

        return Response(content=json.dumps({
            "status": "ok",
            "embed_model": model[0],
            "provider": C.provider,
            "available_providers": list(embed_providers.keys()),
            "ongoing": C.status_ongoing.get('default', {})
        }))

    async def _files_stats(self, request: Request):
        x_token = request.headers.get('X-Auth-Token')

        files_cnt = C.c_session.execute('SELECT COUNT(*) FROM files_full_text;').one()['count']
        chunks_cnt = C.c_session.execute('SELECT COUNT(*) FROM file_chunks_text;').one()['count']

        return Response(content=json.dumps(
            {"files_cnt": files_cnt, "chunks_cnt": chunks_cnt}
        ))

    async def _update_provider(self, data: VecDBUpdateProvider, request: Request):
        x_token = request.headers.get('X-Auth-Token')
        C.provider = data.provider
        C.vecdb_update_required = True
        C.status_ongoing.setdefault('default', {})
        C.status_ongoing['default']['indexing'] = {'status': 'scheduled'}

        def update_embeddings():
            for batch in on_model_change_update_embeddings(data.batch_size):
                C.status_ongoing['default']['indexing'] = {
                    'status': 'in progress',
                    'progress': batch
                }
                yield json.dumps(batch)
            C.status_ongoing['default']['indexing'] = {'status': 'loading vecdb'}
            load_vecdb()
            C.status_ongoing['default']['indexing'] = {'status': 'done'}

        return StreamingResponse(update_embeddings())


class FindRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(FindRouter, self).__init__(*args, **kwargs)
        super(FindRouter, self).add_api_route("/v1/find", self._search, methods=["POST"])

    async def _search(self, data: FindQuery, request: Request):
        x_token = request.headers.get('X-Auth-Token')

        if C.vecdb_update_required:
            load_vecdb()

        embeddings = list(C.encoder.encode([
            c for c in itertools.chain.from_iterable(C.encoder.chunkify(data.query))
        ]))

        ids, scores = C.vecdb.search(embeddings, data.top_k)
        query: Dict[str, FileChunksText] = {
            q.id: q for q in FileChunksText.filter(id__in=list(set(itertools.chain(*ids))))
        }
        result = [
            {
                'file_path': query[uid].name,
                'file_name': Path(query[uid].name).name,
                'text': query[uid].text,
                'score': str(round(score, 3))
            }
            for uid_batch, scores_batch in zip(ids, scores)
            for uid, score in zip(uid_batch, scores_batch)
        ]

        return Response(content=json.dumps({
            "results": result
        }))


class FilesUpdateRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(FilesUpdateRouter, self).__init__(*args, **kwargs)
        super(FilesUpdateRouter, self).add_api_route("/v1/bulk_upload", self._bulk_upload, methods=["POST"])
        super(FilesUpdateRouter, self).add_api_route("/v1/delete_all", self._delete_all, methods=["GET"])

    async def _bulk_upload(self, data: FilesBulkUpload, request: Request):
        x_token = request.headers.get('X-Auth-Token')

        print('uploading files...')
        C.status_ongoing.setdefault('default', {})
        C.status_ongoing['default']['indexing'] = {'status': 'scheduled'}
        final_batch: bool = data.step == data.total

        files = [FileUpload(*f) for f in data.files]
        inserted_f_cnt = insert_files(files)

        C.vecdb_update_required = True
        C.status_ongoing['default']['indexing'] = {
            'status': 'in progress',
            'progress': {'step': str(data.step), 'total': str(data.total)}
        }
        if final_batch and inserted_f_cnt:
            C.status_ongoing['default']['indexing'] = {'status': 'loading vecdb'}
            load_vecdb()
        if final_batch:
            C.status_ongoing['default']['indexing'] = {'status': 'done'}
        return Response(status_code=200)

    async def _delete_all(self, request: Request):
        x_token = request.headers.get('X-Auth-Token')

        delete_all_records()
        return Response(status_code=200)

