from fastapi import APIRouter, Request


class PluginsRouter(APIRouter):

    def __init__(self):
        super().__init__()
        self.plugins = [
            {"label": "Model Hosting", "tab": "model-hosting", "id": "default"},
            {"label": "Stats", "tab": "stats"},
            {"label": "Projects", "tab": "upload"},
            {"label": "Finetune", "tab": "finetune"},
            {"label": "Chat", "tab": "chat"},
            {"label": "Credentials", "tab": "settings", "hamburger": True},
            {"label": "Server Logs", "tab": "server-logs", "hamburger": True},
            {"label": "About", "tab": "about", "hamburger": True},
        ]
        self.add_api_route("/list-plugins", self._list_plugins, methods=["GET"])

    def _list_plugins(self, request: Request):
        return self.plugins
