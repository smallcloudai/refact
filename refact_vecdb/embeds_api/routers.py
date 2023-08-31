import itertools

from typing import List, Dict

import ujson as json

from more_itertools import chunked
from pydantic import BaseModel
from fastapi import APIRouter, Depends
from fastapi import Response, Request
from fastapi.responses import StreamingResponse

from refact_vecdb.embeds_api.embed_spads import embed_providers
from refact_vecdb.embeds_api.context import CONTEXT as C


__all__ = ["MainRouter"]


class TextsForEmbed(BaseModel):
    files: List[Dict[str, str]]
    provider: str
    is_index: str


class MainRouter(APIRouter):
    def __init__(
            self, *args, **kwargs):
        super(MainRouter, self).__init__(*args, **kwargs)
        super(MainRouter, self).add_api_route("/v1/embed", self._embed, methods=["POST"])
        super(MainRouter, self).add_api_route("/v1/providers", self._providers, methods=["GET"])

    async def _embed(self, request: Request, data: TextsForEmbed = Depends()):
        print(f'EMBED: provider={data.provider}, is_index={data.is_index}')
        provider, is_index = data.provider, data.is_index == 'True'
        if provider not in embed_providers:
            return Response(status_code=501, content=f"Unknown provider: {provider}")

        if is_index:
            model = C.models[f'{provider}_index']
        else:
            model = C.models[f'{provider}_search']

        def results_streamer():
            for batch in chunked(data.files, 1):
                model.throw_task(batch)
                for res in model.result():
                    print(f'yielding {len(res)} results which is: {type(res)}; chunk_idx: {res.get("chunk_idx")}')
                    yield json.dumps(res)

        return StreamingResponse(results_streamer())

    async def _providers(self, request: Request):
        return Response(content=json.dumps({'providers': list(embed_providers.keys())}), media_type="application/json")


