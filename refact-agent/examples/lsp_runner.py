import os
import asyncio
import subprocess

from typing import Optional


class LSPServerRunner:
    def __init__(self, repo_path: str, port: int):
        base_command = os.environ["REFACT_LSP_BASE_COMMAND"]
        self._command = [
            *base_command.split(" "),
            "--logs-stderr", f"--http-port={port}",
            f"--workspace-folder={repo_path}",
            "--ast",
        ]

        self._port: int = port
        self._lsp_server: Optional[asyncio.subprocess.Process] = None

    @property
    def _is_lsp_server_running(self) -> bool:
        return self._lsp_server is not None and self._lsp_server.returncode is None

    async def start(self):
        self._lsp_server = await asyncio.create_subprocess_exec(
            *self._command, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)

        while True:
            stderr = await self._lsp_server.stderr.readline()
            if "AST COMPLETE" in stderr.decode():
                break
            if not self._is_lsp_server_running:
                raise RuntimeError(f"LSP server unexpectedly exited, bb")
            await asyncio.sleep(0.01)
        assert self._is_lsp_server_running

    async def stop(self):
        if self._lsp_server is not None:
            self._lsp_server.terminate()
            await self._lsp_server.wait()
        assert not self._is_lsp_server_running

    async def __aenter__(self):
        await self.start()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.stop()
