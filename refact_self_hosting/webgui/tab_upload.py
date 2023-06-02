import time, json, termcolor, os
import asyncio
import aiohttp
from fastapi import APIRouter, Request, Query, Header, File, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse
from pydantic import BaseModel, Required
from refact_self_hosting.webgui.selfhost_webutils import log
from typing import Dict, List, Optional, Any
from refact_self_hosting import env


router = APIRouter()


@router.get("/tab-files-get")
async def tab_files_get(request: Request):
    result = {
        "uploaded_files": {}
    }
    uploaded_path = env.DIR_UNPACKED
    if os.path.isfile(env.CONFIG_HOW_TO_PROCESS):
        config = json.load(open(env.CONFIG_HOW_TO_PROCESS, "r"))
    else:
        config = {'uploaded_files': {}}
    if os.path.isfile(env.CONFIG_PROCESSING_STATS):
        stats = json.load(open(env.CONFIG_PROCESSING_STATS, "r"))
        stats_uploaded_files = stats.get("uploaded_files", {})
    else:
        stats = {"uploaded_files": {}}
        stats_uploaded_files = {}
    default = {
        "which_set": "train",
        "to_db": True,
    }
    for fn in sorted(os.listdir(uploaded_path)):
        result["uploaded_files"][fn] = {
            "which_set": config["uploaded_files"].get(fn, default)["which_set"],
            "to_db": config["uploaded_files"].get(fn, default)["to_db"],
            **stats_uploaded_files.get(fn, {})
        }
    del stats["uploaded_files"]
    result.update(stats)
    return Response(json.dumps(result, indent=4) + "\n")


class TabSingleFileConfig(BaseModel):
    which_set: str = Query(default=Required, regex="train|test")
    to_db: bool


class TabFilesConfig(BaseModel):
    uploaded_files: Dict[str, TabSingleFileConfig]


@router.post("/tab-files-save-config")
async def tab_files_save_config(config: TabFilesConfig):
    with open(env.CONFIG_HOW_TO_PROCESS, "w") as f:
        json.dump(config.dict(), f, indent=4)


@router.post("/tab-files-upload")
async def tab_files_upload(request: Request, file: UploadFile):
    tmp_path = os.path.join(env.DIR_UPLOADS, file.filename + ".tmp")
    file_path = os.path.join(env.DIR_UPLOADS, file.filename)
    if os.path.exists(file_path):
        return Response("File with this name already exists", status_code=409)
    try:
        with open(tmp_path, "wb") as f:
            while True:
                contents = await file.read(1024)
                if not contents:
                    break
                f.write(contents)
        os.rename(tmp_path, file_path)
    except OSError as e:
        response_data = {"message": f"Error: {e}"}
        return JSONResponse(content=response_data, status_code=500)
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)
    return JSONResponse("OK")


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
    last_path_element = os.path.split(post.url)[1]
    file_path = os.path.join(env.DIR_UPLOADS, last_path_element)
    try:
        with open(file_path, "wb") as f:
            f.write(bin)
    except OSError as e:
        return JSONResponse(content={"message": f"Error: {e}"}, status_code=500)
    return JSONResponse("OK")


class CloneRepo(BaseModel):
    url: str
    branch: Optional[str] = None


@router.post("/tab-repo-upload")
async def tab_repo_upload(request: Request, repo: CloneRepo):
    try:
        branch_args = ["-b", repo.branch] if repo.branch else []
        proc = await asyncio.create_subprocess_exec(
            "git", "-C", env.DIR_UPLOADS, "clone", "--no-recursive",
            "--depth", "1", *branch_args, repo.url,
            stdout=asyncio.subprocess.DEVNULL,
            stderr=asyncio.subprocess.PIPE)
        _, stderr = await proc.communicate()
        if proc.returncode != 0:
            raise RuntimeError(stderr.decode())
    except Exception as e:
        return JSONResponse(content={"message": f"Error: {e}"}, status_code=500)
    return JSONResponse("OK")


@router.post("/tab-files-delete")
async def tab_files_delete(request: Request):
    file_name = await request.json()
    file_path = os.path.join(env.DIR_UPLOADS, file_name)
    try:
        os.remove(file_path)
        return JSONResponse("OK")

    except OSError as e:
        return JSONResponse(content={"message": f"Error: {e}"}, status_code=500)

@router.post("/tab-files-process-now")
async def upload_files_process_now(request: Request):
    with open(env.FLAG_LAUNCH_PROCESS_UPLOADS, "w") as f:
        f.write("1")
    return JSONResponse("OK")
