from typing import Callable

from fastapi.requests import Request
from starlette.middleware.base import BaseHTTPMiddleware
from starlette.responses import JSONResponse

from refact_webgui.webgui.selfhost_database import StatisticsService


class StatsMiddleware(BaseHTTPMiddleware):

    def __init__(self,
                 stats_service: StatisticsService,
                 *args, **kwargs):
        self._stats_service = stats_service
        super().__init__(*args, **kwargs)

    async def dispatch(self, request: Request, call_next: Callable):
        if request.url.path.startswith("/stats") and not self._stats_service.is_ready:
            return JSONResponse(
                status_code=500,
                content={"reason": "Statistics service is not ready, waiting for database connection"})
        return await call_next(request)
