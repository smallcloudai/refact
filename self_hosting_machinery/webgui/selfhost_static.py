import os
from fastapi import APIRouter, HTTPException
from fastapi.responses import FileResponse


__all__ = ["StaticRouter"]


static_folders = [
os.path.join(os.path.dirname(os.path.abspath(__file__)), "static")
]


class StaticRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        super().add_api_route("/", self._index, methods=["GET"])
        super().add_api_route("/{file_path:path}", self._static_file, methods=["GET"])
        super().add_api_route("/ping", self._ping_handler, methods=["GET"])

    async def _index(self):
        for spath in static_folders:
            fn = os.path.join(spath, "index.html")
            if os.path.exists(fn):
                return FileResponse(fn, media_type="text/html")
        raise HTTPException(404, "No index.html found")

    async def _static_file(self, file_path: str):
        if ".." in file_path:
            raise HTTPException(404, "Path \"%s\" not found" % file_path)
        for spath in static_folders:
            fn = os.path.join(spath, file_path)
            if os.path.exists(fn):
                return FileResponse(fn)
        raise HTTPException(404, "Path \"%s\" not found" % file_path)

    async def _ping_handler(self):
        return {"message": "pong"}
