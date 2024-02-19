import asyncio
import os.path

from fastapi import APIRouter
from fastapi.responses import JSONResponse

from refact_webgui.webgui.selfhost_webutils import log

from typing import List, Tuple


__all__ = ["TabAboutRouter"]


class TabAboutRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._version_table = None
        self.add_api_route("/tab-about-version-get", self._tab_about_version_get, methods=["GET"])

    async def _get_pip_module_version(self, module_name: str):
        try:
            process = await asyncio.create_subprocess_exec(
                "pip", "show", module_name,
                stdout=asyncio.subprocess.PIPE)
            stdout, stderr = await process.communicate()
            for line in stdout.decode().splitlines():
                if line.startswith("Version:"):
                    return line.split()[-1]
        except Exception as e:
            log(f"Error while getting '{module_name}' version: {e}")
        return "N/A"

    async def _get_lsp_version(self):
        try:
            process = await asyncio.create_subprocess_exec(
                "refact-lsp", "--version",
                stdout=asyncio.subprocess.PIPE)
            stdout, stderr = await process.communicate()
            for line in stdout.decode().splitlines():
                return line.split()[-1]
        except Exception as e:
            log(f"Error while getting 'refact-lsp' version: {e}")
        return "N/A"

    def _get_build_info(self):
        build_info_filename = "/refact-build-info.txt"
        build_info = dict()
        if os.path.exists(build_info_filename):
            with open(build_info_filename, "r") as f:
                build_info = dict(line.split() for line in f.readlines())
        return build_info

    async def _init_version_table(self) -> List[Tuple[str, str, str]]:
        build_info = self._get_build_info()
        refact_version = await self._get_pip_module_version("refact-self-hosting")
        lsp_version = await self._get_lsp_version()
        return [
            ("refact", refact_version, build_info.get("refact", "N/A")),
            ("refact-lsp", lsp_version, build_info.get("refact-lsp", "N/A")),
        ]

    async def _tab_about_version_get(self):
        if not self._version_table:
            self._version_table = await self._init_version_table()
        return JSONResponse({"version_table": self._version_table})
