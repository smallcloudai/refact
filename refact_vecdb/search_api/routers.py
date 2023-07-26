import uuid
import itertools

from pathlib import Path
from typing import Dict, List, Optional

from pydantic import BaseModel
from fastapi import APIRouter, BackgroundTasks
from fastapi import Response, Request

from refact_vecdb.common.crud import get_account_data, update_account_data
from refact_vecdb.common.context import CONTEXT as C
from refact_vecdb.common.vecdb import load_vecdb
from refact_vecdb import VDBEmbeddingsAPI


__all__ = ['MainRouter', 'SearchQuery']


class StatusQuery(BaseModel):
    account: str


class SearchQuery(BaseModel):
    texts: List[str]
    account: str
    top_k: int = 3

    def clamp(self):
        return {
            'texts': self.texts,
            'account': self.account,
            'top_k': self.top_k
        }


class UpdateIndexes(BaseModel):
    account: str
    provider: Optional[str] = None


def account_exists(account: str) -> bool:
    if get_account_data(account):
        return True
    return False


async def update_indexes(data: UpdateIndexes):
    load_vecdb(data.account)


class MainRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super(MainRouter, self).__init__(*args, **kwargs)
        self.add_api_route("/v1/update-indexes", self._update_indexes, methods=["POST"])
        super(MainRouter, self).add_api_route("/v1/status", self._status, methods=["POST"])
        super(MainRouter, self).add_api_route("/v1/files-stats", self._files_stats, methods=["POST"])
        super(MainRouter, self).add_api_route("/v1/search", self._search, methods=["POST"])

    async def _update_indexes(self, data: UpdateIndexes, request: Request, background_tasks: BackgroundTasks):
        account = data.account
        if not account_exists(account):
            return Response(status_code=200, content=f"Account {account} not found")
        background_tasks.add_task(update_indexes, data)

    async def _status(self, data: StatusQuery, request: Request):
        account = data.account
        if not account_exists(account):
            update_account_data({'account': account})

        provider = get_account_data(account).get('provider', 'gte')
        return {"provider": provider}

    async def _files_stats(self, data: StatusQuery, request: Request):
        account = data.account
        if not account_exists(account):
            update_account_data({'account': account})

        session = C.c_session

        files_cnt = session.execute(
            session.prepare('SELECT COUNT(*) FROM files_full_text where account = ?'),
            [account]
        ).one()['count']

        chunks_cnt = session.execute(
            session.prepare('SELECT COUNT(*) FROM file_chunks_text where account =?'),
            [account]
        ).one()['count']

        return {"files_cnt": files_cnt, "chunks_cnt": chunks_cnt}

    async def _search(self, data: SearchQuery, request: Request):
        account = data.account
        if not account_exists(account):
            return Response(status_code=200, content=f"Account {account} not found")

        account_data = get_account_data(account)
        emb_api = VDBEmbeddingsAPI()
        provider = account_data.get('provider', 'gte')

        embeddings = []
        results = await emb_api.a_create(
            texts=[{'name': str(uuid.uuid4())[:12], 'text': text} for text in data.texts],
            provider=provider
        )
        for result in results:
            embeddings.append(result['embedding'])
        ids, scores = C.vecdb[account].search(embeddings, data.top_k)

        file_chunks_text = C.c_models['file_chunks_text']
        query: Dict = {
            q.id: q for q in file_chunks_text.filter(id__in=list(set(itertools.chain(*ids))), account=account)
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
        return {'results': results}
