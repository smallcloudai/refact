import json
import os
import asyncio
import aiohttp
import time
import shutil

from fastapi import APIRouter, Request, Query, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse, StreamingResponse

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
    to_db: bool = Query(default=False)


class TabFilesConfig(BaseModel):
    uploaded_files: Dict[str, TabSingleFileConfig]


class FileTypesSetup(BaseModel):
    filetypes_finetune: Dict[str, bool] = Query(default={})
    filetypes_db: Dict[str, bool] = Query(default={})
    force_include: str = Query(default="")
    force_exclude: str = Query(default="")


class TabFilesDeleteEntry(BaseModel):
    delete_this: str = Query(default=Required, regex=r'^(?!.*\/)(?!.*\.\.)[\s\S]+$')


class TabUploadRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-files-get", self._tab_files_get, methods=["GET"])
        self.add_api_route("/tab-files-save-config", self._tab_files_save_config, methods=["POST"])
        self.add_api_route("/tab-files-upload", self._tab_files_upload, methods=["POST"])
        self.add_api_route("/tab-files-upload-url", self._upload_file_from_url, methods=["POST"])
        self.add_api_route("/tab-files-repo-upload", self._tab_files_repo_upload, methods=["POST"])
        self.add_api_route("/tab-files-delete", self._tab_files_delete, methods=["POST"])
        self.add_api_route("/tab-files-process-now", self._upload_files_process_now, methods=["GET"])
        self.add_api_route("/tab-files-filetypes-setup", self._tab_files_filetypes_setup, methods=["POST"])
        self.add_api_route("/tab-files-log", self._tab_files_log, methods=["GET"])

    async def _tab_files_get(self):
        result = {
            "uploaded_files": {}
        }
        uploaded_path = env.DIR_UPLOADS
        if os.path.isfile(env.CONFIG_HOW_TO_UNZIP):
            how_to_process = json.load(open(env.CONFIG_HOW_TO_UNZIP, "r"))
        else:
            how_to_process = {'uploaded_files': {}}
        if os.path.isfile(env.CONFIG_HOW_TO_FILETYPES):
            result["filetypes"] = json.load(open(env.CONFIG_HOW_TO_FILETYPES, "r"))
        else:
            result["filetypes"] = {}
        if os.path.isfile(env.CONFIG_PROCESSING_STATS):
            scan_stats = json.load(open(env.CONFIG_PROCESSING_STATS, "r"))
            mtime = os.path.getmtime(env.CONFIG_PROCESSING_STATS)
            stats_uploaded_files = scan_stats.get("uploaded_files", {})
            any_working = False
            for fstat in stats_uploaded_files.values():
                if fstat["status"] in ["working", "starting"]:
                    any_working = True
                    if mtime + 600 < time.time():
                        fstat["status"] = "failed"
            scan_stats["filtering_stage"] = (0 if any_working else 1)
        else:
            scan_stats = {"uploaded_files": {}, "filtering_stage": 0}
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
                with open(os.path.join(uploaded_path, fn, GIT_CONFIG_FILENAME)) as f:
                    config = json.load(f)
                result["uploaded_files"][fn].update({
                    "is_git": True,
                    **config
                })
        del scan_stats["uploaded_files"]

        result.update(scan_stats)
        result["filestats_ftf"] = {}
        if os.path.isfile(env.CONFIG_FINETUNE_FILTER_STATS):
            result["filestats_ftf"] = json.load(open(env.CONFIG_FINETUNE_FILTER_STATS))
        # 0 new zip
        # 1 files done, pick file types
        # 2 gpu filtering done
        return Response(json.dumps(result, indent=4) + "\n")

    async def _tab_files_save_config(self, config: TabFilesConfig):
        with open(env.CONFIG_HOW_TO_UNZIP + ".tmp", "w") as f:
            json.dump(config.dict(), f, indent=4)
        os.rename(env.CONFIG_HOW_TO_UNZIP + ".tmp", env.CONFIG_HOW_TO_UNZIP)
        # _reset_process_stats()  -- this requires process script restart, but it flashes too much in GUI
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
            log("Error while uploading file: %s" % (e or str(type(e))))
            return JSONResponse({"message": "Cannot upload file, see logs for details"}, status_code=500)
        finally:
            if os.path.exists(tmp_path):
                os.remove(tmp_path)
        _reset_process_stats()
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
        _reset_process_stats()
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
        _reset_process_stats()
        return JSONResponse("OK")

    async def _tab_files_delete(self, request: Request, delete_entry: TabFilesDeleteEntry):
        file_path = os.path.join(env.DIR_UPLOADS, delete_entry.delete_this)
        try:
            os.unlink(file_path)
        except OSError as e:
            pass
        try:
            shutil.rmtree(file_path)
        except OSError as e:
            pass
        _reset_process_stats()
        return JSONResponse("OK")

    async def _tab_files_log(self, phase: str, accepted_or_rejected: str):
        fn = ""
        if phase == "finetune_filter":
            if accepted_or_rejected == "accepted":
                fn = env.LOG_FILES_ACCEPTED_FTF
            else:
                fn = env.LOG_FILES_REJECTED_FTF
        else:
            if accepted_or_rejected == "accepted":
                fn = env.LOG_FILES_ACCEPTED_SCAN
            else:
                fn = env.LOG_FILES_REJECTED_SCAN
        if os.path.isfile(fn):
            return StreamingResponse(
                stream_text_file(fn),
                media_type="text/plain"
            )
        else:
            return Response("File list empty\n", media_type="text/plain")

    async def _tab_files_filetypes_setup(self, post: FileTypesSetup):
        with open(env.CONFIG_HOW_TO_FILETYPES + ".tmp", "w") as f:
            json.dump(post.dict(), f, indent=4)
        os.rename(env.CONFIG_HOW_TO_FILETYPES + ".tmp", env.CONFIG_HOW_TO_FILETYPES)
        _start_process_now()
        return JSONResponse("OK")

    async def _upload_files_process_now(self, upto_filtering_stage: int = Query(0)):
        _start_process_now()
        return JSONResponse("OK")


def _start_process_now():
    with open(env.FLAG_LAUNCH_PROCESS_UPLOADS, "w") as f:
        f.write("")
    _reset_process_stats()


def _reset_process_stats():
    try:
        os.remove(env.CONFIG_PROCESSING_STATS)
    except OSError as e:
        pass


async def stream_text_file(fn):
    f = open(fn, "r")
    while True:
        line = f.readline()
        if not line:
            break
        yield line
