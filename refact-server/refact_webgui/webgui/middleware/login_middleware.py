from typing import Callable

from fastapi.requests import Request
from fastapi.responses import RedirectResponse
from starlette.middleware.base import BaseHTTPMiddleware

from refact_webgui.webgui.selfhost_login import RefactSession


class LoginMiddleware(BaseHTTPMiddleware):

    def __init__(self,
                 session: RefactSession,
                 *args, **kwargs):
        self._session = session
        super().__init__(*args, **kwargs)

    async def dispatch(self, request: Request, call_next: Callable):
        if any(map(request.url.path.startswith, self._session.exclude_routes)) \
                or self._session.authenticate(request.cookies.get("session_key")):
            return await call_next(request)
        return RedirectResponse(url="/admin")
