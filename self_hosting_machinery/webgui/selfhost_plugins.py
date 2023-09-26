import json
from fastapi import APIRouter, Request


class PluginsRouter(APIRouter):

    def __init__(self):
        super().__init__()
        self.plugins = [
            {"label": "Model Hosting", "tab": "model-hosting"},
            {"label": "Sources", "tab": "upload"},
            {"label": "Finetune", "tab": "finetune"},
            {"label": "Server Logs", "tab": "server-logs"},
            {"label": "Credentials", "tab": "settings", "hamburger": True},
        ]
        self.add_api_route("/list-plugins", self._list_plugins, methods=["GET"])

    def _list_plugins(self, request: Request):
        return self.plugins
