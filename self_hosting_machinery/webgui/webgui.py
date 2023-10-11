import logging
import asyncio

import uvloop
import sys
import signal
import uvicorn
import weakref

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from starlette.middleware.base import BaseHTTPMiddleware

from self_hosting_machinery.webgui.selfhost_model_assigner import ModelAssigner
from self_hosting_machinery.webgui.selfhost_plugins import PluginsRouter
from self_hosting_machinery.webgui.selfhost_req_queue import Ticket
from self_hosting_machinery.webgui.selfhost_fastapi_completions import CompletionsRouter
from self_hosting_machinery.webgui.selfhost_fastapi_gpu import GPURouter
from self_hosting_machinery.webgui.tab_server_logs import TabServerLogRouter
from self_hosting_machinery.webgui.tab_settings import TabSettingsRouter
from self_hosting_machinery.webgui.tab_upload import TabUploadRouter
from self_hosting_machinery.webgui.tab_finetune import TabFinetuneRouter
from self_hosting_machinery.webgui.tab_models_host import TabHostRouter
from self_hosting_machinery.webgui.selfhost_queue import InferenceQueue
from self_hosting_machinery.webgui.selfhost_static import StaticRouter
from self_hosting_machinery.webgui.tab_loras import TabLorasRouter

from typing import Dict


class WebGUI(FastAPI):

    def __init__(self, model_assigner: ModelAssigner, *args, **kwargs):
        super().__init__(*args, **kwargs)

        inference_queue = InferenceQueue()
        id2ticket: Dict[str, Ticket] = weakref.WeakValueDictionary()
        for router in self._routers_list(id2ticket, inference_queue, model_assigner):
            self.include_router(router)

        class NoCacheMiddleware(BaseHTTPMiddleware):
            async def dispatch(self, request, call_next):
                response = await call_next(request)
                response.headers["Cache-Control"] = "no-cache"
                return response

        self.add_middleware(
            CORSMiddleware,
            allow_origins=[],
            allow_credentials=True,
            allow_methods=["*"],
            allow_headers=["*"],
        )
        self.add_middleware(NoCacheMiddleware)

        self.add_event_handler("startup", self._startup_event)

    @staticmethod
    def _routers_list(
            id2ticket: Dict[str, Ticket],
            inference_queue: InferenceQueue,
            model_assigner: ModelAssigner):
        return [
            TabLorasRouter(),
            PluginsRouter(),
            CompletionsRouter(
                prefix="/v1",
                id2ticket=id2ticket,
                inference_queue=inference_queue,
                model_assigner=model_assigner),
            GPURouter(
                prefix="/infengine-v1",
                id2ticket=id2ticket,
                inference_queue=inference_queue),
            TabServerLogRouter(),
            TabUploadRouter(),
            TabFinetuneRouter(
                models_db=model_assigner.models_db),
            TabHostRouter(model_assigner),
            TabSettingsRouter(model_assigner),
            StaticRouter(),
        ]

    async def _startup_event(self):
        def handle_sigint(*args):
            print("Received SIGINT or SIGUSR1, exiting...")
            exit(1)

        signal.signal(signal.SIGINT, handle_sigint)
        signal.signal(signal.SIGUSR1, handle_sigint)


if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--host", default="0.0.0.0")
    parser.add_argument("--port", default=8008, type=int)
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s WEBUI %(message)s',
        datefmt='%Y%m%d %H:%M:%S',
        handlers=[logging.StreamHandler(stream=sys.stderr)])

    model_assigner = ModelAssigner()
    app = WebGUI(model_assigner, docs_url=None, redoc_url=None)

    asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
    uvicorn.run(
        app, host=args.host, port=args.port,
        timeout_keep_alive=600, log_config=None)
