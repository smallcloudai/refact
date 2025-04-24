import os
import asyncio
from typing import Dict

import uvicorn
import uvloop

from fastapi import APIRouter, Request

from refact_webgui.webgui.selfhost_database import RefactDatabase
from refact_webgui.webgui.selfhost_database import StatisticsService
from refact_webgui.webgui.selfhost_login import AdminRouter
from refact_webgui.webgui.selfhost_login import AdminSession
from refact_webgui.webgui.selfhost_login import DummySession
from refact_webgui.webgui.selfhost_login import RefactSession
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner
from refact_webgui.webgui.selfhost_queue import InferenceQueue, Ticket
from refact_webgui.webgui.selfhost_static import StaticRouter
from refact_webgui.webgui.selfhost_statistics import TabStatisticsRouter
from refact_webgui.webgui.tab_about import TabAboutRouter
from refact_webgui.webgui.tab_server_logs import TabServerLogRouter
from refact_webgui.webgui.tab_third_party_apis import TabThirdPartyApisRouter
from refact_webgui.webgui.webgui import WebGUI
from refact_webgui.webgui.webgui import setup_logger

from refact_proxy.webgui.selfhost_model_assigner import ProxyModelAssigner
from refact_proxy.webgui.selfhost_fastapi_completions import ProxyCompletionsRouter


class ProxyPluginsRouter(APIRouter):

    def __init__(self):
        super().__init__()
        self.plugins = [
            {"label": "Third-Party APIs", "tab": "third-party-apis"},
            # NOTE: there are no completion models on server for now, so no need in stats
            # {"label": "Stats", "tab": "stats"},
            # TODO: there is no watchdog, so no logs
            # {"label": "Server Logs", "tab": "server-logs", "hamburger": True},
            {"label": "About", "tab": "about", "hamburger": True},
        ]
        self.add_api_route("/list-plugins", self._list_plugins, methods=["GET"])

    def _list_plugins(self, _request: Request):
        return self.plugins


class ProxyWebGUI(WebGUI):

    @staticmethod
    def _routers_list(
            id2ticket: Dict[str, Ticket],
            inference_queue: InferenceQueue,
            model_assigner: ModelAssigner,
            stats_service: StatisticsService,
            session: RefactSession):
        return [
            ProxyPluginsRouter(),
            AdminRouter(
                prefix="/admin",
                session=session),
            TabThirdPartyApisRouter(),
            ProxyCompletionsRouter(
                id2ticket=id2ticket,
                inference_queue=inference_queue,
                model_assigner=model_assigner,
                session=session),
            TabStatisticsRouter(
                prefix="/stats",
                stats_service=stats_service,
                session=session,
            ),
            TabServerLogRouter(),
            TabAboutRouter(),
            StaticRouter(),
        ]


if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--host", default="0.0.0.0")
    parser.add_argument("--port", default=8008, type=int)
    args = parser.parse_args()
    setup_logger()

    model_assigner = ProxyModelAssigner()
    database = RefactDatabase()
    stats_service = StatisticsService(database)

    admin_token = os.environ.get("REFACT_ADMIN_TOKEN", None)
    session = AdminSession(admin_token) if admin_token is not None else DummySession()

    app = ProxyWebGUI(
        model_assigner=model_assigner,
        database=database,
        stats_service=stats_service,
        session=session,
        docs_url=None, redoc_url=None)

    asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
    uvicorn.run(
        app, host=args.host, port=args.port,
        timeout_keep_alive=600, log_config=None)
