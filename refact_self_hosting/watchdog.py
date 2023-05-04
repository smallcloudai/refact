import os
import sys
import time
import logging
import signal
import subprocess

from datetime import datetime
from pathlib import Path

from typing import Optional


class Watchdog:

    def __init__(self,
                 port: int,
                 workdir: str,
                 token: str,
                 disable_updates: bool,
                 failed_upgrade_quit: bool = False):
        self._port = port
        self._workdir = workdir
        self._token = token
        self._failed_upgrade_quit = failed_upgrade_quit
        self._disable_updates = disable_updates

        self._quit_flag = False
        self._package1_url = "git+https://github.com/smallcloudai/code-contrast.git"
        self._package2_url = "git+https://github.com/smallcloudai/refact-self-hosting.git"

        signal.signal(signal.SIGUSR1, self._catch_sigkill)

    def _update_package(self, package_url) -> bool:
        try:
            subprocess.check_output(
                [sys.executable, "-m", "pip", "install", "--upgrade", "--no-cache-dir", package_url],
                stderr=subprocess.DEVNULL)
            package_info = subprocess.check_output(
                [sys.executable, "-m", "pip", "show", "code-contrast"],
                stderr=subprocess.DEVNULL)
            logging.info(f"package updated")
            for info in package_info.decode("utf8").split("\n"):
                logging.info(info)
            return True
        except subprocess.CalledProcessError as e:
            logging.error(e)
            return False

    def _start_server(self) -> Optional[subprocess.Popen]:
        try:
            command = [
                sys.executable,
                "-m",
                "refact_self_hosting.server",
                f"--port={self._port}",
                f"--workdir={self._workdir}",
                f"--token={self._token}",
            ]
            process = subprocess.Popen(
                command,
                stdout=sys.stdout,
                stderr=sys.stderr,
            )
            logging.info(f"server started")
            return process
        except ValueError as e:
            logging.error(e)
            return None

    def _catch_sigkill(self, signum, frame):
        logging.info("caught SIGUSR1")
        self._quit_flag = True

    def run(self):
        while not self._quit_flag:
            if not self._disable_updates:
                successful = self._update_package(self._package1_url)
                if self._failed_upgrade_quit and not successful:
                    break
                successful = self._update_package(self._package2_url)
                if self._failed_upgrade_quit and not successful:
                    break
            else:
                logging.info('DISABLE_UPDATES is set, not updating')
            process = self._start_server()
            while True:
                if self._quit_flag:
                    process.kill()
                    logging.info(f"server is shutting down")
                    process.wait()
                    break
                retcode = process.poll()
                if retcode is not None:
                    logging.info(f"server exited with {retcode}")
                    break
                time.sleep(10)


if __name__ == "__main__":
    workdir = str(os.environ.get("SERVER_WORKDIR"))
    port = int(os.environ.get("SERVER_PORT"))
    token = os.environ.get("SERVER_API_TOKEN", None)
    disable_updates = bool(os.environ.get("DISABLE_UPDATES", False))

    logdir = Path(workdir) / "logs"
    logdir.mkdir(exist_ok=True, parents=False)
    file_handler = logging.FileHandler(filename=logdir / f"watchdog_{datetime.now():%Y-%m-%d-%H-%M-%S}.log")
    stream_handler = logging.StreamHandler(stream=sys.stdout)
    logging.basicConfig(level=logging.INFO, handlers=[stream_handler, file_handler])

    logging.basicConfig(level=logging.INFO,
                        format='%(asctime)s - %(message)s',
                        datefmt='%Y-%m-%d %H:%M:%S')

    watchdog = Watchdog(
        port=port,
        workdir=workdir,
        token=token,
        disable_updates=disable_updates
    )
    watchdog.run()
