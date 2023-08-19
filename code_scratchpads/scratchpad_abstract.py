from typing import Dict, AsyncGenerator, Any
import logging
import time


debug_logger = logging.getLogger("SCRATCHPAD").debug


class Scratchpad:
    def __init__(
        self,
        *,
        tokenizer,
        request_created_ts: float,
    ):
        self.tokenizer = tokenizer
        self.finish_reason = ""
        self.request_created_ts = request_created_ts

    def prompt(self, context_size: int, sampling_parameters_to_patch: Dict[str, Any]):
        raise NotImplementedError()

    async def re_stream_response(self, text_generator: AsyncGenerator[Any, None]):
        raise NotImplementedError()

    def _debuglog(self, msg):
        debug_logger("%4.0fms %s" % (1000*(time.time() - self.request_created_ts), msg))
