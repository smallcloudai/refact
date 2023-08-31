import json
import uuid
import itertools

from pathlib import Path
from typing import Dict, List

from pydantic import BaseModel
from fastapi import APIRouter
from fastapi import Response, Request

from refact_vecdb.search_api.context import CONTEXT as C
from refact_vecdb import VDBEmbeddingsAPI


__all__ = ['MainRouter']


class StatusQuery(BaseModel):
    keyspace: str


class SearchQuery(BaseModel):
    texts: List[str]
    keyspace: str
    top_k: int = 3


class MainRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(MainRouter, self).__init__(*args, **kwargs)
        super(MainRouter, self).add_api_route("/v1/status", self._status, methods=["POST"])
        super(MainRouter, self).add_api_route("/v1/files-stats", self._files_stats, methods=["POST"])
        super(MainRouter, self).add_api_route("/v1/search", self._search, methods=["POST"])

    async def _status(self, data: StatusQuery, request: Request):
        if data.keyspace not in C.c_sessions:
            return Response(content=json.dumps({"error": f"Unknown keyspace: {data.keyspace}"}))

        provider = C.c_sessions[data.keyspace]['provider']
        return Response(content=json.dumps(
            {"provider": provider})
        )

    async def _files_stats(self, data: StatusQuery, request: Request):
        if data.keyspace not in C.c_sessions:
            return Response(content=json.dumps({"error": f"Unknown keyspace: {data.keyspace}"}))

        session = C.c_sessions[data.keyspace]['session']

        files_cnt = session.execute('SELECT COUNT(*) FROM files_full_text;').one()['count']
        chunks_cnt = session.execute('SELECT COUNT(*) FROM file_chunks_text;').one()['count']

        return Response(content=json.dumps(
            {"files_cnt": files_cnt, "chunks_cnt": chunks_cnt}
        ))

    async def _search(self, data: SearchQuery, request: Request):
        if data.keyspace not in C.c_sessions:
            return Response(content=json.dumps({"error": f"Unknown keyspace: {data.keyspace}"}))

        vdb_api = VDBEmbeddingsAPI()
        provider = C.c_sessions[data.keyspace]['provider']
        embeddings = []
        async for result in vdb_api.a_create(
            texts=[{'name': str(uuid.uuid4())[:12], 'text': text} for text in data.texts],
            provider=provider,
            is_index='False'
        ):
            embeddings.append(result['embedding'])
        ids, scores = C.c_sessions[data.keyspace]['vecdb'].search(embeddings, data.top_k)

        file_chunks_text = C.c_sessions[data.keyspace]['models']['file_chunks_text']
        query: Dict = {
            q.id: q for q in file_chunks_text.filter(id__in=list(set(itertools.chain(*ids))))
        }

        results = [
            {
                'file_path': query[uid].name,
                'file_name': Path(query[uid].name).name,
                'text': query[uid].text,
                'score': str(round(score, 3))
            }
            for uid_batch, scores_batch in zip(ids, scores)
            for uid, score in zip(uid_batch, scores_batch)
        ]

        return Response(content=json.dumps({"results": results}))
