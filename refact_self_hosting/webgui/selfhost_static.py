import os
from fastapi import APIRouter, HTTPException
from fastapi.responses import FileResponse


__all__ = ["StaticRouter"]


class StaticRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        super().add_api_route("/", self._index, methods=["GET"])
        super().add_api_route("/{file_path:path}", self._static_file, methods=["GET"])
        super().add_api_route("/ping", self._ping_handler, methods=["GET"])
        self._this_file_dir = os.path.dirname(os.path.abspath(__file__))

    async def _index(self):
        html_path = os.path.join(self._this_file_dir, "static", "index.html")
        return FileResponse(html_path, media_type="text/html")

    async def _static_file(self, file_path: str):
        if ".." in file_path:
            raise HTTPException(404, "Path \"%s\" not found" % file_path)
        static_path = os.path.join(self._this_file_dir, "static", file_path)
        if not os.path.exists(static_path):
            raise HTTPException(404, "Path \"%s\" not found" % file_path)
        return FileResponse(static_path)

    async def _ping_handler(self):
        return {"message": "pong"}
