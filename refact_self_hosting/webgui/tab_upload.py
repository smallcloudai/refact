import json
import os
import asyncio
import aiohttp
import time
import shutil

from fastapi import APIRouter, Request, Query, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse, StreamingResponse
from refact_data_pipeline import finetune_filtering_defaults

from refact_self_hosting import env
from refact_self_hosting.env import GIT_CONFIG_FILENAME, get_all_ssh_keys
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
    which_set: str = Query(default=Required, regex="auto|train|test")
    to_db: bool


class TabFilesConfig(BaseModel):
    uploaded_files: Dict[str, TabSingleFileConfig]


class FilteringSetup(BaseModel):
    filter_loss_threshold: Optional[float] = Query(default=None, gt=2, le=10)
    filter_gradcosine_threshold: Optional[float] = Query(default=None, gt=-1.0, le=0.5)
    limit_train_files: Optional[int] = Query(default=None, gt=20, le=10000)
    limit_time_seconds: Optional[int] = Query(default=None, gt=300, le=3600*6)
    include_file_types: Dict[str, bool] = Query(default={})
    force_include: str = Query(default="")
    force_exclude: str = Query(default="")
    use_gpus_n: Optional[int] = Query(default=False, gt=1, le=8)


class TabFilesDeleteEntry(BaseModel):
    delete_this: str = Query(default=Required, regex=r'^(?!.*\/)(?!.*\.\.)[\s\S]+$')


class TabUploadRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-files-get", self._tab_files_get, methods=["GET"])
        self.add_api_route("/tab-files-save-config", self._tab_files_save_config, methods=["POST"])
        self.add_api_route("/tab-files-upload", self._tab_files_upload, methods=["POST"])
        self.add_api_route("/tab-files-upload-url", self._upload_file_from_url, methods=["POST"])
        self.add_api_route("/tab-files-rejected", self._tab_files_rejected, methods=["GET"])
        self.add_api_route("/tab-files-repo-upload", self._tab_files_repo_upload, methods=["POST"])
        self.add_api_route("/tab-files-delete", self._tab_files_delete, methods=["POST"])
        self.add_api_route("/tab-files-process-now", self._upload_files_process_now, methods=["GET"])
        self.add_api_route("/tab-files-setup-filtering", self._tab_files_setup_filtering, methods=["POST"])

    async def _tab_files_get(self):
        result = {
            "uploaded_files": {}
        }
        uploaded_path = env.DIR_UPLOADS
        if os.path.isfile(env.CONFIG_HOW_TO_UNZIP):
            how_to_process = json.load(open(env.CONFIG_HOW_TO_UNZIP, "r"))
        else:
            how_to_process = {'uploaded_files': {}}
        if os.path.isfile(env.CONFIG_HOW_TO_FILTER):
            result["filter_setup"] = json.load(open(env.CONFIG_HOW_TO_FILTER, "r"))
        else:
            result["filter_setup"] = {}
        result["filter_setup_defaults"] = finetune_filtering_defaults.finetune_filtering_defaults
        if os.path.isfile(env.CONFIG_PROCESSING_STATS):
            stats = json.load(open(env.CONFIG_PROCESSING_STATS, "r"))
            mtime = os.path.getmtime(env.CONFIG_PROCESSING_STATS)
            stats_uploaded_files = stats.get("uploaded_files", {})
            for fstat in stats_uploaded_files.values():
                if fstat["status"] in ["working", "starting"]:
                    if mtime + 600 < time.time():
                        fstat["status"] = "failed"
        else:
            stats = {"uploaded_files": {}}
            stats_uploaded_files = {}
        default = {
            "which_set": "train",
            "to_db": True,
        }
        for fn in sorted(os.listdir(uploaded_path)):
            result["uploaded_files"][fn] = {
                "which_set": how_to_process["uploaded_files"].get(fn, default)["which_set"],
                "to_db": how_to_process["uploaded_files"].get(fn, default)["to_db"],
                "is_git": False,
                **stats_uploaded_files.get(fn, {})
            }
            if os.path.exists(os.path.join(uploaded_path, fn, GIT_CONFIG_FILENAME)):
                with open(os.path.join(uploaded_path, fn, GIT_CONFIG_FILENAME), 'r') as f:
                    config = json.load(f)
                result["uploaded_files"][fn].update({
                    "is_git": True,
                    **config
                })

        del stats["uploaded_files"]
        result.update(stats)
        result["filtering_stage"] = 0
        # 0 new zip
        # 1 files done, pick file types
        # 2 gpu filtering done
        return Response(json.dumps(result, indent=4) + "\n")

    async def _tab_files_save_config(self, config: TabFilesConfig):
        with open(env.CONFIG_HOW_TO_UNZIP + ".tmp", "w") as f:
            json.dump(config.dict(), f, indent=4)
        os.rename(env.CONFIG_HOW_TO_UNZIP + ".tmp", env.CONFIG_HOW_TO_UNZIP)
        return JSONResponse("OK")

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

    def _make_git_command(self):
        command = ['ssh', '-o', 'UserKnownHostsFile=/dev/null', '-o', 'StrictHostKeyChecking=no']
        for ssh_key in get_all_ssh_keys():
            command += ['-i', ssh_key]
        return ' '.join(command)

    async def _tab_files_repo_upload(self, repo: CloneRepo):
        def get_repo_name_from_url(url: str) -> str:
            last_slash_index = url.rfind("/")
            last_suffix_index = url.rfind(".git")
            if last_suffix_index < 0:
                last_suffix_index = len(url)

            if last_slash_index < 0 or last_suffix_index <= last_slash_index:
                raise Exception("Badly formatted url {}".format(url))

            return url[last_slash_index + 1:last_suffix_index]
        try:
            repo_name = get_repo_name_from_url(repo.url)
            repo_base_dir = os.path.join(env.DIR_UPLOADS, repo_name)
            os.makedirs(repo_base_dir, exist_ok=False)
            with open(os.path.join(repo_base_dir, GIT_CONFIG_FILENAME), 'w') as f:
                json.dump({
                    "url": repo.url,
                    "branch": repo.branch,
                }, f)
        except FileExistsError as _:
            return JSONResponse({"message": f"Error: {repo_name} is exist"}, status_code=500)
        except Exception as e:
            return JSONResponse({"message": f"Error: {e}"}, status_code=500)
        return JSONResponse("OK")

    async def _tab_files_delete(self, request: Request, delete_entry: TabFilesDeleteEntry):
        file_path = os.path.join(env.DIR_UPLOADS, delete_entry.delete_this)
        try:
            shutil.rmtree(file_path)
            return JSONResponse("OK")

        except OSError as e:
            log("Error deleting file: %s" % e)
            return JSONResponse({"message": f"Error: {e}"}, status_code=500)

    async def _tab_files_rejected(self, request: Request):
        file_path = os.path.join(env.DIR_UNPACKED, "files_rejected.log")
        if os.path.isfile(file_path):
            return StreamingResponse(
                stream_text_file(file_path),
                media_type="text/plain"
            )
        else:
            return Response("No files rejecetd", media_type="text/plain")

    async def _tab_files_setup_filtering(self, post: FilteringSetup):
        validated = post.dict()
        for dkey, dval in finetune_filtering_defaults.finetune_filtering_defaults.items():
            if dkey in validated and (validated[dkey] == dval or validated[dkey] is None):
                del validated[dkey]
        with open(env.CONFIG_HOW_TO_FILTER + ".tmp", "w") as f:
            json.dump(post.dict(), f, indent=4)
        os.rename(env.CONFIG_HOW_TO_FILTER + ".tmp", env.CONFIG_HOW_TO_FILTER)
        return JSONResponse("OK")

    async def _upload_files_process_now(self, upto_filtering_stage: int = Query(0)):
        with open(env.FLAG_LAUNCH_PROCESS_UPLOADS, "w") as f:
            f.write("1")
        try:
            os.remove(env.CONFIG_PROCESSING_STATS)
        except OSError as e:
            pass
        return JSONResponse("OK")


async def stream_text_file(fn):
    f = open(fn, "r")
    while True:
        line = f.readline()
        if not line:
            break
        yield line
