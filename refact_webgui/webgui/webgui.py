import os
import re
import logging
import asyncio

import uvloop
import sys
import signal
import uvicorn
import weakref

from fastapi import FastAPI
from fastapi.requests import Request
from fastapi.responses import RedirectResponse
from fastapi.responses import JSONResponse
from fastapi.middleware.cors import CORSMiddleware
from starlette.middleware.base import BaseHTTPMiddleware

from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner
from refact_webgui.webgui.selfhost_plugins import PluginsRouter
from refact_webgui.webgui.selfhost_fastapi_completions import CompletionsRouter
from refact_webgui.webgui.selfhost_fastapi_gpu import GPURouter
from refact_webgui.webgui.tab_server_logs import TabServerLogRouter
from refact_webgui.webgui.tab_settings import TabSettingsRouter
from refact_webgui.webgui.tab_toolbox import TabToolboxRouter
from refact_webgui.webgui.tab_upload import TabUploadRouter
from refact_webgui.webgui.tab_finetune import TabFinetuneRouter
from refact_webgui.webgui.tab_models_host import TabHostRouter
from refact_webgui.webgui.selfhost_queue import InferenceQueue, Ticket
from refact_webgui.webgui.selfhost_static import StaticRouter
from refact_webgui.webgui.tab_loras import TabLorasRouter
from refact_webgui.webgui.selfhost_statistics import TabStatisticsRouter
from refact_webgui.webgui.selfhost_login import LoginRouter
from refact_webgui.webgui.tab_about import TabAboutRouter

from refact_webgui.webgui.selfhost_database import RefactDatabase
from refact_webgui.webgui.selfhost_database import StatisticsService
from refact_webgui.webgui.selfhost_lsp_proxy import LspProxy
from refact_webgui.webgui.selfhost_login import RefactSession
from refact_webgui.webgui.selfhost_login import DummySession
from refact_webgui.webgui.selfhost_login import AdminSession

from typing import Dict, Callable


class WebGUI(FastAPI):

    def __init__(self,
                 model_assigner: ModelAssigner,
                 database: RefactDatabase,
                 stats_service: StatisticsService,
                 session: RefactSession,
                 *args, **kwargs):
        super().__init__(*args, **kwargs)

        self._model_assigner = model_assigner
        self._database = database
        self._stats_service = stats_service
        self._session = session

        inference_queue = InferenceQueue()
        id2ticket: Dict[str, Ticket] = weakref.WeakValueDictionary()
        for router in self._routers_list(
                id2ticket,
                inference_queue,
                self._model_assigner,
                self._stats_service,
                self._session):
            self.include_router(router)

        class NoCacheMiddleware(BaseHTTPMiddleware):
            async def dispatch(self, request, call_next):
                response = await call_next(request)
                response.headers["Cache-Control"] = "no-cache"
                return response

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
                return RedirectResponse(url="/login")

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

        self.add_middleware(
            CORSMiddleware,
            allow_origins=[],
            allow_credentials=True,
            allow_methods=["*"],
            allow_headers=["*"],
        )
        self.add_middleware(NoCacheMiddleware)
        self.add_middleware(
            LoginMiddleware,
            session=self._session,
        )
        self.add_middleware(
            StatsMiddleware,
            stats_service=self._stats_service,
        )

        self.add_event_handler("startup", self._startup_event)

    @staticmethod
    def _routers_list(
            id2ticket: Dict[str, Ticket],
            inference_queue: InferenceQueue,
            model_assigner: ModelAssigner,
            stats_service: StatisticsService,
            session: RefactSession):
        return [
            TabLorasRouter(),
            PluginsRouter(),
            LoginRouter(
                prefix="/login",
                session=session),
            TabStatisticsRouter(
                prefix="/stats",
                stats_service=stats_service,
                session=session,
            ),
            CompletionsRouter(
                id2ticket=id2ticket,
                inference_queue=inference_queue,
                model_assigner=model_assigner,
                session=session),
            GPURouter(
                prefix="/infengine-v1",
                id2ticket=id2ticket,
                inference_queue=inference_queue),
            TabServerLogRouter(),
            TabUploadRouter(),
            TabToolboxRouter(),
            TabFinetuneRouter(
                model_assigner=model_assigner),
            TabHostRouter(model_assigner),
            TabSettingsRouter(model_assigner),
            LspProxy(session=session),
            TabAboutRouter(),
            StaticRouter(),
        ]

    async def _startup_event(self):
        def handle_sigint(*args):
            print("Received SIGINT or SIGUSR1, exiting...")
            exit(1)

        signal.signal(signal.SIGINT, handle_sigint)
        signal.signal(signal.SIGUSR1, handle_sigint)
        signal.signal(signal.SIGTERM, handle_sigint)

        async def init_database():
            await self._database.connect()
            await self._stats_service.init_models()

        asyncio.create_task(init_database(), name="database_initialization")


def setup_logger():
    # Suppress messages like this:
    # WEBUI 127.0.0.1:55610 - "POST /infengine-v1/completions-wait-batch HTTP/1.1" 200
    # WEBUI 127.0.0.1:41574 - "POST /infengine-v1/completion-upload-results
    boring1 = re.compile("completions-wait-batch.* 200")
    boring2 = re.compile("completion-upload-results.* 200")
    class CustomHandler(logging.Handler):
        def emit(self, record):
            log_entry = self.format(record)
            if boring1.match(log_entry):
                return
            if boring2.match(log_entry):
                return
            sys.stderr.write(log_entry)
            sys.stderr.write("\n")
            sys.stderr.flush()

    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s WEBUI %(message)s',
        datefmt='%Y%m%d %H:%M:%S',
        handlers=[CustomHandler()]
    )


if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--host", default="0.0.0.0")
    parser.add_argument("--port", default=8008, type=int)
    args = parser.parse_args()
    setup_logger()

    model_assigner = ModelAssigner()
    database = RefactDatabase()
    stats_service = StatisticsService(database)

    admin_token = os.environ.get("REFACT_ADMIN_TOKEN", None)
    session = AdminSession(admin_token) if admin_token is not None else DummySession()

    app = WebGUI(
        model_assigner=model_assigner,
        database=database,
        stats_service=stats_service,
        session=session,
        docs_url=None, redoc_url=None)

    asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
    uvicorn.run(
        app, host=args.host, port=args.port,
        timeout_keep_alive=600, log_config=None)
