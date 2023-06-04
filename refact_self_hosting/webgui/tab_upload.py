import json
import os
import asyncio
import aiohttp

from fastapi import APIRouter, Request, Query, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse

from refact_self_hosting import env
from refact_self_hosting.webgui.selfhost_webutils import log

from pydantic import BaseModel, Required
from typing import Dict, Optional


__all__ = ["TabUploadRouter"]


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


class UploadViaURL(BaseModel):
    url: str


class CloneRepo(BaseModel):
    url: str
    branch: Optional[str] = None


class TabSingleFileConfig(BaseModel):
    which_set: str = Query(default=Required, regex="train|test")
    to_db: bool


class TabFilesConfig(BaseModel):
    uploaded_files: Dict[str, TabSingleFileConfig]


class TabUploadRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-files-get", self._tab_files_get, methods=["GET"])
        self.add_api_route("/tab-files-save-config", self._tab_files_save_config, methods=["POST"])
        self.add_api_route("/tab-files-upload", self._tab_files_upload, methods=["POST"])
        self.add_api_route("/tab-files-upload-url", self._upload_file_from_url, methods=["POST"])
        self.add_api_route("/tab-repo-upload", self._tab_repo_upload, methods=["POST"])
        self.add_api_route("/tab-files-delete", self._tab_files_delete, methods=["POST"])
        self.add_api_route("/tab-files-process-now", self._upload_files_process_now, methods=["GET"])

    async def _tab_files_get(self):
        result = {
            "uploaded_files": {}
        }
        uploaded_path = env.DIR_UPLOADS
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

    async def _tab_files_save_config(self, config: TabFilesConfig):
        with open(env.CONFIG_HOW_TO_PROCESS, "w") as f:
            json.dump(config.dict(), f, indent=4)

    async def _tab_files_upload(self, file: UploadFile):
        tmp_path = os.path.join(env.DIR_UPLOADS, file.filename + ".tmp")
        file_path = os.path.join(env.DIR_UPLOADS, file.filename)
        if os.path.exists(file_path):
            response_data = {"message": f"File with this name already exists"}
            return JSONResponse(content=response_data, status_code=409)
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
            return JSONResponse(response_data, status_code=500)
        finally:
            if os.path.exists(tmp_path):
                os.remove(tmp_path)
        return JSONResponse("OK")

    async def _upload_file_from_url(self, post: UploadViaURL):
        log("downloading \"%s\"" % post.url)
        bin = await download_file_from_url(post.url)
        log("/download")
        last_path_element = os.path.split(post.url)[1]
        file_path = os.path.join(env.DIR_UPLOADS, last_path_element)
        try:
            with open(file_path, "wb") as f:
                f.write(bin)
        except OSError as e:
            return JSONResponse({"message": f"Error: {e}"}, status_code=500)
        return JSONResponse("OK")

    async def _tab_repo_upload(self, repo: CloneRepo):
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
            return JSONResponse({"message": f"Error: {e}"}, status_code=500)
        return JSONResponse("OK")

    async def _tab_files_delete(self, request: Request):
        file_name = await request.json()
        file_path = os.path.join(env.DIR_UPLOADS, file_name)
        try:
            os.remove(file_path)
            return JSONResponse("OK")

        except OSError as e:
            return JSONResponse({"message": f"Error: {e}"}, status_code=500)

    async def _upload_files_process_now(self):
        log("set flag %s" % env.FLAG_LAUNCH_PROCESS_UPLOADS)
        with open(env.FLAG_LAUNCH_PROCESS_UPLOADS, "w") as f:
            f.write("1")
        return JSONResponse("OK")
