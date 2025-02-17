import time

from self_hosting_machinery.finetune.utils import traces


class Timer:
    def __init__(self, message: str):
        self._start_time = None
        self._message_template = message

    def __call__(self, message: str):
        self._message_template = message

    def __enter__(self):
        self._start_time = time.time()

    def __exit__(self, exc_type, exc_val, exc_tb):
        elapsed_time = time.time() - self._start_time
        traces.log(self._message_template.format(
            time_s=elapsed_time,
            time_ms=elapsed_time * 1000
        ))
