import time, json, termcolor, os
import asyncio
import aiohttp
from fastapi import APIRouter, Request, Query, Header, File, UploadFile, HTTPException
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
    else:
        config = {'uploaded_files': {}}
    default = {
        "which_set": "train",
        "to_db": True,
    }
    for fn in sorted(os.listdir(uploaded_path)):
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


@router.post("/tab-files-upload")
async def tab_files_upload(request: Request, file: UploadFile):
    file_path = os.path.expanduser("~/data/uploaded_files")
    file_path = os.path.join(file_path, file.filename)
    try:
        with open(file_path, "wb") as f:
            while True:
                contents = await file.read(1024)
                if not contents:
                    break
                f.write(contents)
    except OSError as e:
        return Response(f"Error: {e}")
    return Response("OK")


async def download_file_from_url(url: str):
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            if response.status != 200:
                raise HTTPException(
                    status_code=500,
                    detail=f"Cannot download: {response.reason} {response.status}",
                )
            file = await response.read()
            return file


class FileToDownload(BaseModel):
    url: str


@router.post("/tab-files-upload-url")
async def upload_file_from_url(request: Request, post: FileToDownload):
    log("downloading \"%s\"" % post.url)
    bin = await download_file_from_url(post.url)
    log("/download")
    uploaded_dir = os.path.expanduser("~/data/uploaded_files")
    last_path_element = os.path.split(post.url)[1]
    file_path = os.path.join(uploaded_dir, last_path_element)
    try:
        with open(file_path, "wb") as f:
            f.write(bin)
    except OSError as e:
        return Response(f"Error: {e}")
    return Response("OK")


@router.post("/tab-files-delete")
async def tab_files_delete(request: Request):
    file_name = await request.json()
    file_path = os.path.expanduser("~/data/uploaded_files")
    file_path = os.path.join(file_path, file_name)
    try:
        os.remove(file_path)
        return Response("OK")
    except OSError as e:
        return Response(f"Error: {e}")
