import os
import time
import asyncio
import random
import subprocess

from typing import Optional


__all__ = ["LSPServerRunner"]


def localhost_port_not_in_use(start: int, stop: int):
    def _is_port_in_use(port: int) -> bool:
        import socket
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            return s.connect_ex(('localhost', port)) == 0

    ports_range = list(range(start, stop))
    random.shuffle(ports_range)
    for port in ports_range:
        if not _is_port_in_use(port):
            return port

    raise RuntimeError(f"cannot find port in range [{start}, {stop})")


class LSPServerRunner:
    def __init__(self, repo_path: str, use_ast: bool, use_vecdb: bool):
        base_command = os.environ["REFACT_LSP_BASE_COMMAND"]
        # /Users/valaises/RustroverProjects/refact-lsp/target/debug/refact-lsp --address-url http://localhost:8008 -k MYKEY
        assert base_command, "env REFACT_LSP_BASE_COMMAND must be specified"
        port = localhost_port_not_in_use(8100, 9000)
        self._command = [
            *base_command.split(" "),
            "--logs-stderr", f"--http-port={port}",
            f"--workspace-folder={repo_path}",
        ]
        if use_ast:
            self._command.append("--ast")
        if use_vecdb:
            self._command.append("--vecdb")

        self._use_ast = use_ast
        self._use_vecdb = use_vecdb
        self._port: int = port
        self._lsp_server: Optional[asyncio.subprocess.Process] = None

    @property
    def _is_lsp_server_running(self) -> bool:
        return self._lsp_server is not None and self._lsp_server.returncode is None

    @property
    def base_url(self):
        return f"http://127.0.0.1:{self._port}/v1"

    async def _start(self):
        t0 = time.time()
        print("REFACT LSP start", " ".join(self._command))
        self._lsp_server = await asyncio.create_subprocess_exec(
            *self._command, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        ast_ok, vecdb_ok = False, False
        while True:
            stderr = await self._lsp_server.stderr.readline()
            if "AST COMPLETE" in stderr.decode():
                print("AST initialized")
                ast_ok = True
            if "VECDB COMPLETE" in stderr.decode():
                print("VECDB initialized")
                vecdb_ok = False
            if (self._use_ast == ast_ok) and (self._use_vecdb == vecdb_ok):
                break
            if not self._is_lsp_server_running:
                raise RuntimeError(f"LSP server unexpectedly exited, bb")
            await asyncio.sleep(0.01)
        print("REFACT LSP /start in %0.2fs" % (time.time() - t0))
        assert self._is_lsp_server_running

    async def _stop(self):
        if self._lsp_server is not None:
            print("REFACT LSP STOP")
            try:
                self._lsp_server.terminate()
                try:
                    await asyncio.wait_for(self._lsp_server.wait(), timeout=5.0)
                except asyncio.TimeoutError:
                    print("LSP server did not terminate in time, forcefully killing")
                    self._lsp_server.kill()
                    await self._lsp_server.wait()
            except Exception as e:
                print(f"Error stopping LSP server: {e}")
            finally:
                self._lsp_server = None
            print("REFACT LSP /STOP")

    async def __aenter__(self):
        await self._start()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self._stop()
