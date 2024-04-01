import asyncio
import uuid
import json

import aiofiles
import os
import subprocess
import tempfile

from typing import Union
from pathlib import Path

from fastapi import APIRouter, UploadFile, HTTPException, Query
from fastapi.responses import JSONResponse, StreamingResponse
from pydantic import Required

from refact_utils.scripts import env
from refact_utils.scripts.best_lora import find_best_checkpoint
from refact_webgui.webgui.selfhost_webutils import log
from refact_webgui.webgui.tab_upload import download_file_from_url, UploadViaURL
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner


def rm(f):
    try:
        subprocess.check_call(["rm", "-rf", f])
    except Exception as e:
        log(f"Error while removing file: {e}")


async def unpack(file_path: Path) -> JSONResponse:
    def get_mimetype(fp: Union[str, Path]) -> str:
        m = subprocess.check_output(["file", "--mime-type", "-b", fp])
        return m.decode("utf-8").strip()

    upload_filename = str(file_path)
    unpack_filename = str(file_path.parent)

    if not file_path.is_file():
        return JSONResponse({"detail": f"Error while unpacking: File {file_path.name} does not exist"}, status_code=404)

    try:
        mime_type = get_mimetype(upload_filename)
        if 'application/x-tar' in mime_type:
            cmd = ["tar", "-xf", upload_filename, "-C", unpack_filename]
        elif 'application/x-bzip2' in mime_type:
            cmd = ["tar", "-xjf", upload_filename, "-C", unpack_filename]
        elif 'application/x-gzip' in mime_type:
            cmd = ["tar", "-xzf", upload_filename, "-C", unpack_filename]
        elif 'application/zip' in mime_type:
            cmd = ["unzip", "-q", "-o", upload_filename, "-d", unpack_filename]
        else:
            return JSONResponse({"detail": f"Error while unpacking: Unknown archive type {mime_type}"}, status_code=400)
        subprocess.check_call(cmd)
        rm(os.path.join(unpack_filename, file_path.name))
        return JSONResponse("OK", status_code=200)

    except Exception as e:
        log(f"Error while unpacking: {e}")
        return JSONResponse({"detail": f"Error while unpacking: {e}"}, status_code=500)


class TabLorasRouter(APIRouter):

    def __init__(self, model_assigner: ModelAssigner, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._model_assigner = model_assigner
        self.add_api_route("/lora-upload", self._upload_lora, methods=["POST"])
        self.add_api_route("/lora-upload-url", self._upload_lora_url, methods=["POST"])
        self.add_api_route("/lora-download", self._download_lora, methods=["GET"])
        self.add_api_route("/lora-merge-download", self._download_lora_merge, methods=["GET"])

    async def _upload_lora(self, file: UploadFile):
        async def write_to_file() -> JSONResponse:
            upload_dest = env.DIR_LORAS
            tmp_path = os.path.join(upload_dest, file.filename + ".tmp")
            file_path = os.path.join(upload_dest, file.filename)
            if os.path.exists(file_path):
                return JSONResponse({"detail": f"File with this name already exists"}, status_code=409)
            try:
                with open(tmp_path, "wb") as f:
                    while True:
                        if not (contents := await file.read(1024 * 1024)):
                            break
                        f.write(contents)
                os.rename(tmp_path, file_path)
                return JSONResponse("OK", status_code=200)
            except OSError as e:
                log("Error while uploading file: %s" % (e or str(type(e))))
                return JSONResponse({"detail": "Cannot upload file, see logs for details"}, status_code=500)
            finally:
                try:
                    os.remove(tmp_path)
                except:
                    pass

        f = Path(os.path.join(env.DIR_LORAS, file.filename))

        if (resp := await write_to_file()).status_code != 200:
            rm(f)
            return resp

        if (resp := await unpack(f)).status_code != 200:
            rm(f)
            return resp

        return JSONResponse("OK", status_code=200)

    async def _upload_lora_url(self, file: UploadViaURL):
        try:
            file_path = await download_file_from_url(file.url, env.DIR_LORAS)
        except Exception as e:
            return JSONResponse({"detail": f"Cannot download: {e}"}, status_code=500)

        resp = await unpack(Path(file_path))
        rm(file_path)
        if resp.status_code != 200:
            return resp
        return JSONResponse("OK", status_code=200)

    async def _download_lora(self,
                             run_id: str = Query(default=Required),
                             checkpoint_id: str = Query(default="")):

        async def _archived_content(run_id: str, checkpoint_id: str):
            tempdir = Path(env.TMPDIR) / f"lora-download-{uuid.uuid4()}"
            copy_run_dirname = tempdir / run_id
            zipped_run_filename = tempdir / f"{run_id}.zip"
            try:
                tempdir.mkdir(parents=False, exist_ok=False)

                # copy run to temp_run_dirname
                process = await asyncio.create_subprocess_exec(
                    "cp", "-r", str(Path(env.DIR_LORAS) / run_id), str(copy_run_dirname))
                await process.wait()
                if process.returncode != 0:
                    raise RuntimeError("run copying failed")

                # remove unspecified checkpoints
                checkpoints_dir = copy_run_dirname / "checkpoints"
                for checkpoint_dir in checkpoints_dir.iterdir():
                    if checkpoint_dir.name != checkpoint_id:
                        rm(str(checkpoint_dir))

                # zip prepared run
                process = await asyncio.create_subprocess_exec(
                    "zip", "-r", str(zipped_run_filename), run_id,
                    cwd=str(zipped_run_filename.parent))
                await process.wait()
                if process.returncode != 0:
                    raise RuntimeError("archive creation failed")

                async with aiofiles.open(zipped_run_filename, "rb") as f:
                    while True:
                        if not (contents := await f.read(1024 * 1024)):
                            break
                        yield contents

                rm(str(tempdir))

            except BaseException as e:
                rm(str(tempdir))
                err_msg = "Error while downloading: %s" % (e or str(type(e)))
                log(err_msg)
                raise HTTPException(detail=err_msg, status_code=500)

            finally:
                rm(str(tempdir))

        download_filename = run_id + (f"-{checkpoint_id}" if checkpoint_id else "") + ".zip"
        if not checkpoint_id:
            checkpoint_id = find_best_checkpoint(run_id)["best_checkpoint_id"]

        return StreamingResponse(
            _archived_content(run_id, checkpoint_id),
            media_type="application/x-zip-compressed",
            headers={
                "Content-Type": "application/x-zip-compressed",
                "Content-Disposition": f'attachment; filename={download_filename}',
            })

    async def _download_lora_merge(self, run_id: str = Query(default=Required), checkpoint_id: str = Query(default="")):

        async def _archived_content(model_path: str, checkpoint_path: Path):
            try:
                with tempfile.TemporaryDirectory() as tempdir:
                    output_filename = Path(tempdir) / f"{uuid.uuid4()}.zip"
                    process = await asyncio.create_subprocess_exec(
                        "python", "-m", "refact_utils.scripts.merge_lora",
                        model_path, str(checkpoint_path), str(output_filename))
                    await process.wait()
                    if process.returncode != 0:
                        raise RuntimeError(f"run copying failed")

                    async with aiofiles.open(output_filename, "rb") as f:
                        while True:
                            if not (contents := await f.read(128 * 1024 * 1024)):
                                break
                            yield contents

            except Exception as e:
                err_msg = f"Error while lora merge download: {e or str(type(e))}"
                log(err_msg)
                raise HTTPException(detail=err_msg, status_code=500)

        run_path = Path(env.DIR_LORAS) / run_id
        try:
            config_filename = run_path / "config.json"
            if not config_filename.exists():
                raise RuntimeError(f"Run {run_id} config does not exist")
            with config_filename.open("r") as f:
                model_name = json.load(f)["model_name"]
            model_path = self._model_assigner.models_db[model_name]["model_path"]
        except Exception as e:
            log(f"Lora merge failed '{run_id}': {e}")
            raise HTTPException(detail=f"Lora merge failed '{run_id}': {e}", status_code=500)

        if not checkpoint_id:
            checkpoint_id = find_best_checkpoint(run_id)["best_checkpoint_id"]
        checkpoint_path = run_path / "checkpoints" / checkpoint_id
        download_filename = f"{model_name}-{run_id}-{checkpoint_id}.zip"

        return StreamingResponse(
            _archived_content(model_path, checkpoint_path),
            media_type="application/x-zip-compressed",
            headers={
                "Content-Type": "application/x-zip-compressed",
                "Content-Disposition": f'attachment; filename={download_filename}',
            })
