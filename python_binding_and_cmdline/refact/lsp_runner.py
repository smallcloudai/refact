import os
import time
import asyncio
import random
import subprocess
from typing import Optional, List


class LSPServerRunner:
    def __init__(
        self,
        refact_lsp_command: List[str],  # all parameters except --logs-stderr and --http-port that will be added mandatory for this class to work
        wait_for_ast_vecdb: bool,
        refact_lsp_log: Optional[str],
        verbose: bool,
    ):
        self._refact_lsp_command = refact_lsp_command
        self._refact_lsp_log = refact_lsp_log
        self._refact_lsp_process: Optional[asyncio.subprocess.Process] = None
        self._port: int = 0
        self._wait_for_ast_vecdb = wait_for_ast_vecdb
        self._verbose = verbose

    def check_if_still_running(self) -> bool:
        return self._refact_lsp_process is not None and self._refact_lsp_process.returncode is None

    def base_url(self):
        return f"http://127.0.0.1:{self._port}/v1"

    async def start(self):
        assert self._refact_lsp_process is None
        ports_tried = []
        for maybe_port_busy in range(5):
            self._port = random.randint(8100, 9100)
            program = self._refact_lsp_command[0]
            args = [
                *self._refact_lsp_command[1:],
                "--logs-stderr",
                f"--http-port={self._port}",
            ]
            ports_tried.append(self._port)
            wait_ast = ("--ast" in args) and self._wait_for_ast_vecdb
            wait_vecdb = ("--vecdb" in args) and self._wait_for_ast_vecdb

            t0 = time.time()
            if self._verbose:
                print("REFACT LSP start", program, " ".join(args))
            self._refact_lsp_process = await asyncio.create_subprocess_exec(program, *args, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            ast_ok, vecdb_ok, post_listening, post_busy = False, False, False, False
            while True:
                while True:
                    stderr_line = await self._query_stderr()
                    if stderr_line is None:
                        break
                    if "HTTP server listening" in stderr_line:
                        post_listening = True
                    if "AST COMPLETE" in stderr_line:
                        ast_ok = True
                    if "VECDB COMPLETE" in stderr_line:
                        vecdb_ok = True
                    if "PORT_BUSY" in stderr_line:
                        post_busy = True
                if (not wait_ast or ast_ok) and (not wait_vecdb or vecdb_ok) and post_listening:
                    break
                if post_busy:
                    break
                if not self.check_if_still_running():
                    print(self._refact_lsp_process)
                    print(self._refact_lsp_process.returncode)
                    raise RuntimeError(f"LSP server exited unexpectedly :/")
                await asyncio.sleep(0.1)  # waiting for start up
            if post_busy:
                if self._verbose:
                    print("REFACT LSP port %d busy" % (self._port))
                await self._stop_real()
                continue
            if self._verbose:
                print("REFACT LSP /start in %0.2fs" % (time.time() - t0))
            self._stderr_task = asyncio.create_task(self._stderr_background_reader())
            break
        else:
            raise RuntimeError(f"After several attempts, couldn't start refact-lsp because it cannot open http port, tried ports {ports_tried}")

    async def _stderr_background_reader(self):
        while self.check_if_still_running():
            while True:
                line = await self._query_stderr()
                if line is None:
                    break
            await asyncio.sleep(0.1)  # waiting for messages, in normal operation

    async def _query_stderr(self):
        if self._refact_lsp_process is None or self._refact_lsp_process.stderr.at_eof():
            return None
        try:
            line = await asyncio.wait_for(self._refact_lsp_process.stderr.readline(), timeout=0.1)
            line = line.decode()
            if "ERR" in line and self._verbose:  # hmm maybe user is interested in errors even without verbose?
                print("REFACT LSP", line.rstrip())
            if self._refact_lsp_log is not None:
                with open(self._refact_lsp_log, "a") as f:
                    f.write(line)
            return line
        except asyncio.TimeoutError:
            return None

    async def _stop_real(self):
        assert self._refact_lsp_process is not None
        try:
            self._refact_lsp_process.terminate()
            try:
                await asyncio.wait_for(self._refact_lsp_process.wait(), timeout=5.0)
            except asyncio.TimeoutError:
                print("LSP server did not terminate in time, forcefully killing")
                self._refact_lsp_process.kill()
                await self._refact_lsp_process.wait()
        except Exception as e:
            print(f"Error stopping LSP server: {e}")
        finally:
            self._refact_lsp_process = None
            self._port = 0

    async def stop(self):
        if self._refact_lsp_process is None:
            return
        if self._verbose:
            print("REFACT LSP stop")
        await self._stop_real()
        if self._verbose:
            print("REFACT LSP /stop")

    async def __aenter__(self):
        await self.start()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.stop()
