from fastapi import FastAPI, HTTPException, APIRouter
from starlette.requests import Request
from starlette.responses import StreamingResponse
from starlette.background import BackgroundTask
from fastapi import APIRouter, HTTPException

import httpx

# TODO: can this be configured?
lsp_address = "http://127.0.0.1:8001"

__all__ = ["LspProxy"]


client = httpx.AsyncClient(base_url=lsp_address)

class LspProxy(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        super().add_route("/lsp/v1/{path:path}", self._reverse_proxy, methods=["GET", "POST"])

    async def _reverse_proxy(self, request: Request):
        ## TODO: handle errors
        ## TODO: append api-key to the query
        path = request.url.path.replace("/lsp", "")
        url = httpx.URL(path=path, query=request.url.query.encode("utf-8"))

        rp_req = client.build_request(
            request.method, url, headers=request.headers.raw, content=await request.body()
        )
        rp_resp = await client.send(rp_req, stream=True)
        return StreamingResponse(
            rp_resp.aiter_raw(),
            status_code=rp_resp.status_code,
            headers=rp_resp.headers,
            background=BackgroundTask(rp_resp.aclose),
        )
