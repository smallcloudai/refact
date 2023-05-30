import time, json, termcolor, os
import asyncio
from fastapi import APIRouter, Request, Query, Header
from fastapi.responses import Response
from pydantic import BaseModel, Required
from refact_self_hosting.webgui import selfhost_req_queue
from refact_self_hosting.webgui.selfhost_webutils import log
from typing import Dict, List, Optional, Any


router = APIRouter()


@router.get("/tab-files-get")
async def tab_files_get(request: Request):
    result = {
        "uploaded_files": {}
    }
    uploaded_path = os.path.expanduser("~/data/uploaded_files")
    cfg_fn = os.path.expanduser("~/data/how_to_process.cfg")
    if os.path.isfile(cfg_fn):
        config = json.load(open(cfg_fn, "r"))
    default = {
        "which_set": "train",
        "to_db": True,
    }
    for fn in os.listdir(uploaded_path):
        result["uploaded_files"][fn] = {
            "which_set": config["uploaded_files"].get(fn, default)["which_set"],
            "to_db": config["uploaded_files"].get(fn, default)["to_db"],
        }
    return Response(json.dumps(result, indent=4) + "\n")


class TabSingleFileConfig(BaseModel):
    which_set: str = Query(default=Required, regex="train|test")
    to_db: bool


class TabFilesConfig(BaseModel):
    uploaded_files: Dict[str, TabSingleFileConfig]


@router.post("/tab-files-save-config")
async def tab_files_save_config(config: TabFilesConfig):
    cfg_fn = os.path.expanduser("~/data/how_to_process.cfg")
    with open(cfg_fn, "w") as f:
        json.dump(config.dict(), f, indent=4)

