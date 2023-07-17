import json
from fastapi import APIRouter, Request


plugins = [
{ "label": 'Model Hosting', "tab": 'model-hosting', "active": True },
{ "label": 'Sources', "tab": 'upload' },
{ "label": 'Finetune', "tab": 'finetune' },
{ "label": 'Server Logs', "tab": 'server-logs' },
{ "label": 'Access Control', "tab": 'access-control' },
{ "label": 'Credentials', "tab": 'settings', "hamburger": True },
]


class PluginsRouter(APIRouter):

    def __init__(self):
        super().__init__()
        self.add_api_route("/list-plugins", self._list_plugins, methods=["GET"])

    def _list_plugins(self, request: Request):
        return plugins
