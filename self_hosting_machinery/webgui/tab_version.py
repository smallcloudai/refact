import os.path

from fastapi import APIRouter
from fastapi.responses import JSONResponse

from typing import List, Tuple


__all__ = ["TabVersionRouter"]


class TabVersionRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._refact_build_info = "/refact-build-info.txt"
        self.add_api_route("/tab-version-get", self._tab_version_get, methods=["GET"])

    async def _get_version_table(self) -> List[Tuple[str, str, str]]:
        build_info = dict()
        if os.path.exists(self._refact_build_info):
            with open(self._refact_build_info, "r") as f:
                build_info = dict(line.split() for line in f.readlines())
        return [
            ("Package", "Version", "Commit Hash"),
            ("refact", "", build_info.get("refact", "N/A")),
            ("refact-lsp", "", build_info.get("refact-lsp", "N/A")),
        ]

    async def _tab_version_get(self):
        return JSONResponse({"version_table": await self._get_version_table()})

