import os
import yaml
import json
from fastapi import APIRouter, HTTPException, Body
from fastapi.responses import FileResponse, JSONResponse
from refact_utils.scripts import env


__all__ = ["TabToolboxRouter"]

class TabToolboxRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/toolbox.yaml", self._toolbox, methods=["GET"])
        self.add_api_route("/tab-toolbox-upload", self._toolbox_upload, methods=["POST"])

    async def _toolbox(self):
        if os.path.exists(env.CONFIG_TOOLBOX):
            return FileResponse(env.CONFIG_TOOLBOX, media_type="text/yaml")
        else:
            raise HTTPException(404, "No toolbox.yaml found")

    async def _toolbox_upload(self, data: str = Body(...)):
        with open(env.CONFIG_TOOLBOX, 'w') as f:
            f.write(data)
        return JSONResponse("OK")