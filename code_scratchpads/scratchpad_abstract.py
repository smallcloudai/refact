from typing import Dict, List, Union, AsyncGenerator
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

    def prompt(self, context_size: int):
        raise NotImplementedError()

    async def re_stream_response(self, text_generator: AsyncGenerator[str, None]):
        raise NotImplementedError()

    def _assert_one_token(self, text: str):
        tokens = self.tokenizer.encode(text)
        assert len(tokens) == 1
        return text

    # def _encode_without_special_tokens(self, txt: str) -> List[int]:
    #     if hasattr(self.tokenizer, "tokenizer_copy_but_does_not_encode_special_tokens"):
    #         t = self.tokenizer.tokenizer_copy_but_does_not_encode_special_tokens
    #     else:
    #         t = self.tokenizer.backend_tokenizer
    #     return t.encode(txt, add_special_tokens=False).ids

    def _debuglog(self, msg):
        debug_logger("%4.0fms %s" % (1000*(time.time() - self.request_created_ts), msg))
