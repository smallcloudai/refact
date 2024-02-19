import os
import json
import httpx

from fastapi import APIRouter
from fastapi.exceptions import HTTPException

from starlette.requests import Request
from starlette.responses import StreamingResponse
from starlette.background import BackgroundTask

from refact_utils.scripts import env
from refact_webgui.webgui.selfhost_login import RefactSession

__all__ = ["LspProxy"]


def get_lsp_url() -> str:
    lsp_cfg = json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "lsp.cfg")))
    lsp_cmdline = lsp_cfg["command_line"]
    lsp_port = "8001"

    if "--port" in lsp_cmdline:
        maybe_port:str = lsp_cmdline[lsp_cmdline.index("--port") + 1]
        if maybe_port.isnumeric():
            lsp_port = maybe_port

    return "http://127.0.0.1:" + lsp_port


class LspProxy(APIRouter):

    def __init__(self, session: RefactSession, *args, **kwargs):
        super().__init__(*args, **kwargs)
        super().add_route("/lsp/v1/caps", self._reverse_proxy, methods=["GET"])
        super().add_route("/lsp/v1/chat", self._reverse_proxy_chat, methods=["POST"])
        lsp_address = get_lsp_url()
        self._session = session
        self._client = httpx.AsyncClient(base_url=lsp_address)

    async def _account_from_bearer(self, authorization: str) -> str:
        try:
            return self._session.header_authenticate(authorization)
        except BaseException as e:
            raise HTTPException(status_code=401, detail=str(e))

    async def _reverse_proxy_chat(self, request: Request):
        account = await self._account_from_bearer(request.headers.get("Authorization", None))
        return await self._reverse_proxy(request)

    async def _reverse_proxy(self, request: Request):
        path = request.url.path.replace("/lsp", "")
        url = httpx.URL(path=path, query=request.url.query.encode("utf-8"))

        rp_req = self._client.build_request(
            request.method, url, headers=request.headers.raw, content=await request.body()
        )
        rp_resp = await self._client.send(rp_req, stream=True)
        return StreamingResponse(
            rp_resp.aiter_raw(),
            status_code=rp_resp.status_code,
            headers=rp_resp.headers,
            background=BackgroundTask(rp_resp.aclose),
        )
