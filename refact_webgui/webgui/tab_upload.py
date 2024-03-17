import re
import os
import json
import time
import shutil
import subprocess
import filelock
from typing import Dict, Optional

import aiohttp

from pydantic import BaseModel, Required
from fastapi import APIRouter, Request, Query, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse, StreamingResponse

from refact_utils.scripts import env
from refact_utils.finetune.utils import get_prog_and_status_for_ui
from refact_webgui.webgui.selfhost_webutils import log


__all__ = ["TabUploadRouter", "download_file_from_url", "UploadViaURL"]


async def download_file_from_url(url: str, download_dir: str, force_filename: Optional[str] = None) -> str:
    def extract_filename() -> str:
        try:
            if not (content_disposition := response.headers.get("Content-Disposition")):
                raise Exception('No "content-disposition" header')
            if not (match := re.search(r'filename=["\']?([^"\';]+)["\']?', content_disposition)):
                raise Exception('Could not extract filename from "content-disposition" header')
            if not (fn := match.group(1)):
                raise Exception('No match in "content-disposition" header')
            return fn.strip()
        except Exception as e:
            log(f'Could not extract filename from {url}: {e}; headers: {response.headers}')
            return os.path.split(url)[1][-20:]

    file_path = None
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                if response.status != 200:
                    raise HTTPException(
                        status_code=500,
                        detail=f"Cannot download: {response.reason} {response.status}",
                    )
                filename = force_filename or extract_filename()
                file_path = os.path.join(download_dir, filename)

                with open(file_path, 'wb') as file:
                    async for chunk in response.content.iter_chunked(1024 * 1024):
                        file.write(chunk)
    except Exception as e:
        log(f"Error while downloading from {url}: {e}")
        try:
            if file_path and os.path.exists(file_path):
                subprocess.check_call(['rm', '-rf', file_path])
        except Exception:
            log(f"Error while removing {file_path}")
        raise e
    return file_path


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


class ProjectNameOnly(BaseModel):
    pname: str = Query(default=Required, regex=r'^[A-Za-z0-9_\-\.]+$')


class TabUploadRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-project-list", self._tab_project_list, methods=["GET"])
        self.add_api_route("/tab-project-new", self._tab_project_new, methods=["POST"])
        self.add_api_route("/tab-files-get/{pname}", self._tab_files_get, methods=["GET"])
        self.add_api_route("/tab-files-save-config/{pname}", self._tab_files_save_config, methods=["POST"])
        self.add_api_route("/tab-files-upload/{pname}", self._tab_files_upload, methods=["POST"])
        self.add_api_route("/tab-files-upload-url/{pname}", self._upload_file_from_url, methods=["POST"])
        self.add_api_route("/tab-files-repo-upload/{pname}", self._tab_files_repo_upload, methods=["POST"])
        self.add_api_route("/tab-files-delete/{pname}", self._tab_files_delete, methods=["POST"])
        self.add_api_route("/tab-files-process-now/{pname}", self._upload_files_process_now, methods=["GET"])
        self.add_api_route("/tab-files-filetypes-setup/{pname}", self._tab_files_filetypes_setup, methods=["POST"])
        self.add_api_route("/tab-files-log/{pname}", self._tab_files_log, methods=["GET"])

    async def _tab_project_new(self, project: ProjectNameOnly):
        if not project.pname:
            raise HTTPException(status_code=400, detail="Project name not provided")
        if os.path.exists(env.PP_DIR_UPLOADS(project.pname)):
            raise HTTPException(status_code=400, detail="Project already exists")
        os.makedirs(env.PP_DIR_UPLOADS(project.pname))
        os.makedirs(env.PP_DIR_UNPACKED(project.pname))
        return Response(json.dumps({"status": "success"}, indent=4) + "\n")

    async def _tab_project_list(self):
        projects_list = os.listdir(env.DIR_PROJECTS)
        projects_list = [p for p in projects_list if re.match(r'^[A-Za-z0-9_\-\.]+$', p)]
        if len(projects_list) == 0:
            projects_list = [{"name": "Project1"}]
        else:
            projects_list = [{"name": p} for p in projects_list]
        return Response(json.dumps({
            "projects": projects_list,
        }, indent=4) + "\n")

    async def _tab_files_get(self, pname):
        result = {
            "uploaded_files": {}
        }
        uploaded_path = env.PP_DIR_UPLOADS(pname)
        if os.path.isfile(env.PP_CONFIG_HOW_TO_UNZIP(pname)):
            how_to_process = json.load(open(env.PP_CONFIG_HOW_TO_UNZIP(pname), "r"))
        else:
            how_to_process = {'uploaded_files': {}}

        scan_stats = {"uploaded_files": {}}
        stats_uploaded_files = {}
        disable_gui = True
        if os.path.isfile(env.PP_CONFIG_PROCESSING_STATS(pname)):
            disable_gui = False
            scan_stats = json.load(open(env.PP_CONFIG_PROCESSING_STATS(pname), "r"))
            mtime = os.path.getmtime(env.PP_CONFIG_PROCESSING_STATS(pname))
            stats_uploaded_files = scan_stats.get("uploaded_files", {})
            for fstat in stats_uploaded_files.values():
                if fstat["status"] in ["working", "starting"]:
                    if mtime + 600 < time.time():
                        fstat["status"] = "failed"

        if os.path.isfile(env.PP_CONFIG_HOW_TO_FILETYPES(pname)):
            result["filetypes"] = json.load(open(env.PP_CONFIG_HOW_TO_FILETYPES(pname), "r"))
        else:
            result["filetypes"] = {
                "filetypes_finetune": {},
                "filetypes_db": {},
                "force_include": "",
                "force_exclude": "",
            }

        default = {
            "which_set": "train",
            "to_db": True,
        }
        uploaded_files = os.listdir(uploaded_path)
        if len(uploaded_files) == 0:
            disable_gui = False
        for fn in sorted(uploaded_files):
            result["uploaded_files"][fn] = {
                "which_set": how_to_process["uploaded_files"].get(fn, default)["which_set"],
                "to_db": how_to_process["uploaded_files"].get(fn, default)["to_db"],
                "is_git": False,
                **stats_uploaded_files.get(fn, {})
            }
            if os.path.exists(os.path.join(uploaded_path, fn, "git_config.json")):
                with open(os.path.join(uploaded_path, fn, "git_config.json")) as f:
                    config = json.load(f)
                result["uploaded_files"][fn].update({
                    "is_git": True,
                    **config
                })
        del scan_stats["uploaded_files"]

        result.update(scan_stats)

        prog, status = get_prog_and_status_for_ui(pname)
        working = status in ["starting", "working"]
        result["finetune_working_now"] = False   # TODO remove
        result["disable_ui"] = disable_gui
        return Response(json.dumps(result, indent=4) + "\n")

    async def _tab_files_save_config(self, pname, config: TabFilesConfig):
        log("to save sources config locking project \"%s\"" % pname)
        with filelock.FileLock(env.PP_PROJECT_LOCK(pname)):
            with open(env.PP_CONFIG_HOW_TO_UNZIP(pname) + ".tmp", "w") as f:
                json.dump(config.dict(), f, indent=4)
            os.rename(env.PP_CONFIG_HOW_TO_UNZIP(pname) + ".tmp", env.PP_CONFIG_HOW_TO_UNZIP(pname))
        # _reset_process_stats(pname)  -- this requires process script restart, but it flashes too much in GUI
        return JSONResponse("OK")

    async def _tab_files_upload(self, pname, file: UploadFile):
        log("to upload \"%s\" locking project \"%s\"" % (file.filename, pname))
        with filelock.FileLock(env.PP_PROJECT_LOCK(pname)):
            tmp_path = os.path.join(env.PP_DIR_UPLOADS(pname), file.filename + ".tmp")
            file_path = os.path.join(env.PP_DIR_UPLOADS(pname), file.filename)
            if os.path.exists(file_path):
                response_data = {"message": f"File with this name already exists"}
                return JSONResponse(content=response_data, status_code=409)
            try:
                with open(tmp_path, "wb") as f:
                    while True:
                        contents = await file.read(1024 * 1024)
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

        _start_process_now(pname)
        return JSONResponse("OK")

    async def _upload_file_from_url(self, pname, post: UploadViaURL):
        log("to downloading url \"%s\" locking project \"%s\"" % (post.url, pname))
        with filelock.FileLock(env.PP_PROJECT_LOCK(pname)):
            try:
                await download_file_from_url(post.url, env.PP_DIR_UPLOADS(pname))
            except Exception as e:
                return JSONResponse({"message": f"Cannot download: {e}"}, status_code=500)
            log("/download")
        # _reset_process_stats(pname)
        _start_process_now(pname)
        return JSONResponse("OK")

    def _make_git_command(self):
        command = ['ssh', '-o', 'UserKnownHostsFile=/dev/null', '-o', 'StrictHostKeyChecking=no']
        for ssh_key in env.get_all_ssh_keys():
            command += ['-i', ssh_key]
        return ' '.join(command)

    async def _tab_files_repo_upload(self, pname, repo: CloneRepo):
        class IncorrectUrl(Exception):
            def __init__(self):
                super().__init__()
        def cleanup_url(url: str):
            for sym in ["\t", " "]:
                splited = list(filter(lambda x: len(x) > 0, url.split(sym)))
                if len(splited) > 1:
                    raise IncorrectUrl()
                url = splited[0]
            return url
        def check_url(url: str):
            from giturlparse import parse
            if not parse(url).valid:
                raise IncorrectUrl()
            return url

        def get_repo_name_from_url(url: str) -> str:
            if url.endswith('/'):
                url = url[:-1]
            last_slash_index = url.rfind("/")
            last_suffix_index = url.rfind(".git")
            if last_suffix_index < 0:
                last_suffix_index = len(url)

            if last_slash_index < 0 or last_suffix_index <= last_slash_index:
                raise Exception("Badly formatted url {}".format(url))

            return url[last_slash_index + 1:last_suffix_index]
        log("to clone repo \"%s\" locking project \"%s\"" % (repo.url, pname))
        with filelock.FileLock(env.PP_PROJECT_LOCK(pname)):
            try:
                url = cleanup_url(repo.url)
                url = check_url(url)
                repo_name = get_repo_name_from_url(url)
                repo_base_dir = os.path.join(env.PP_DIR_UPLOADS(pname), repo_name)
                os.makedirs(repo_base_dir, exist_ok=False)
                with open(os.path.join(repo_base_dir, "git_config.json"), 'w') as f:
                    json.dump({
                        "url": url,
                        "branch": repo.branch,
                    }, f)
            except FileExistsError as _:
                return JSONResponse({"message": f"Error: {repo_name} exists"}, status_code=500)
            except IncorrectUrl:
                return JSONResponse({"message": f"Error: incorrect url"}, status_code=500)
            except Exception as e:
                return JSONResponse({"message": f"Error: {e}"}, status_code=500)
        _start_process_now(pname, git_pull=True)   # git_pull is not necessary, but it useful for git call inside process_uploaded_files to show stats immediately (before git clone starts)
        return JSONResponse("OK")

    async def _tab_files_delete(self, pname, request: Request, delete_entry: TabFilesDeleteEntry):
        log("to delete \"%s\" locking project \"%s\"" % (delete_entry.delete_this, pname))
        with filelock.FileLock(env.PP_PROJECT_LOCK(pname)):
            file_path = os.path.join(env.PP_DIR_UPLOADS(pname), delete_entry.delete_this)
            try:
                os.unlink(file_path)
            except OSError as e:
                pass
            try:
                shutil.rmtree(file_path)
            except OSError as e:
                pass
            try:
                # So it starts with the default once files are added again
                if not os.listdir(env.PP_DIR_UPLOADS(pname)) and os.path.exists(env.PP_CONFIG_HOW_TO_FILETYPES(pname)):
                    os.remove(env.PP_CONFIG_HOW_TO_FILETYPES(pname))
            except OSError as e:
                pass
        _start_process_now(pname)
        return JSONResponse("OK")

    async def _tab_files_log(self, pname, accepted_or_rejected: str):
        if accepted_or_rejected == "accepted":
            fn = env.PP_LOG_FILES_ACCEPTED_SCAN(pname)
        else:
            fn = env.PP_LOG_FILES_REJECTED_SCAN(pname)
        if os.path.isfile(fn):
            return StreamingResponse(
                stream_text_file(fn),
                media_type="text/plain"
            )
        else:
            return Response("File list empty\n", media_type="text/plain")

    async def _tab_files_filetypes_setup(self, pname, post: FileTypesSetup):
        log("to save file types config locking project \"%s\"" % pname)
        with filelock.FileLock(env.PP_PROJECT_LOCK(pname)):
            with open(env.PP_CONFIG_HOW_TO_FILETYPES(pname) + ".tmp", "w") as f:
                json.dump(post.dict(), f, indent=4)
            os.rename(env.PP_CONFIG_HOW_TO_FILETYPES(pname) + ".tmp", env.PP_CONFIG_HOW_TO_FILETYPES(pname))
        _start_process_now(pname, dont_delete_stats=True)
        return JSONResponse("OK")

    async def _upload_files_process_now(self, pname: str, git_pull: bool):
        log("_upload_files_process_now for project \"%s\", git_pull=%s" % (pname, git_pull))
        _start_process_now(pname, git_pull=git_pull)
        return JSONResponse("OK")


def _start_process_now(pname: str, dont_delete_stats=False, git_pull=False):
    if not dont_delete_stats:
        _reset_process_stats(pname)
    process_cfg_j = json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "process_uploaded.cfg")))
    fn = os.path.join(env.DIR_WATCHDOG_D, "process_uploaded_%s.cfg" % pname)
    process_cfg_j["save_status"] = env.PP_SCAN_STATUS(pname)
    process_cfg_j["command_line"] += ["--pname", pname]
    if git_pull:
        process_cfg_j["command_line"] += ["--want-pull"]
    del process_cfg_j["unfinished"]
    with open(fn + ".tmp", "w") as f:
        json.dump(process_cfg_j, f, indent=4)
    os.rename(fn + ".tmp", fn)


def _reset_process_stats(pname: str):
    try:
        os.remove(env.PP_CONFIG_PROCESSING_STATS(pname))
    except OSError as e:
        pass


async def stream_text_file(fn):
    f = open(fn, "r")
    while True:
        line = f.readline()
        if not line:
            break
        yield line
