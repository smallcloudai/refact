import os
import sys
import asyncio
import termcolor
import functools
import json
from typing import List, Union, Callable, Dict, Iterator, Tuple

import openai
import tiktoken

from refact_scratchpads import utils as scratchpad_utils
from refact_scratchpads_no_gpu.async_scratchpad import ascratch

from .gpt_chat_spad import gpt_prices, calculate_chat_tokens
from .utils import trim_context_tok, code_block_postprocess


openai.api_key = os.environ.get("OPENAI_API_KEY")
DEBUG = int(os.environ.get("DEBUG", "0"))


@functools.lru_cache(maxsize=10)
def engine_to_encoding(engine: str) -> tiktoken.Encoding:
    enc = tiktoken.encoding_for_model(engine)
    return enc


ACCUMULATE_N_STREAMING_CHUNKS = 5
engine_to_encoding("text-davinci-003")  # this immediately tests if tiktoken works or not


class ScratchpadChatGPT(ascratch.AsyncScratchpad):
    def __init__(
            self,
            id: str,
            created: float,
            temperature: float,
            max_tokens: int,
            stop_tokens: Union[str, List[str]],
            function: str,
            intent: str,
            cursor_file: str,
            cursor0: int,
            cursor1: int,
            sources: Dict[str, str],
            stream: bool,
            logger: Callable,

            model_n: str = "gpt-3.5-turbo",
            supports_stream: bool = True,
            timeout: int = None,
            **kwargs,
    ):
        super().__init__(
            id=id,
            created=created,
            temperature=temperature,
            max_tokens=max_tokens,
            stop_tokens=stop_tokens,
            function=function,
            stream=stream,
            logger=logger,
            **kwargs
        )
        self.intent = intent
        self.cursor_file = cursor_file
        self.cursor0 = cursor0
        self.cursor1 = cursor1
        self.sources = sources
        self.metering_generated_tokens_n = 0
        self.metering_total_tokens_n = 0
        self.needs_upload = False

        self._model_n = model_n
        self.__model_name = None

        if not supports_stream: self.stream = False
        self._stream_timeout_sec: float = 15

        self._txt: str = self.sources.get(self.cursor_file)

        self.cursor0, self.cursor1, self.selection = scratchpad_utils.full_line_selection(
            self.cursor0, self.cursor1, self._txt
        )
        self.enc = engine_to_encoding(self.model_name)

    def trim_context(self) -> Tuple[int, int, str]:
        cursor0, cursor1, ctxt = trim_context_tok(self.cursor0, self.cursor1, self._txt, self.enc)
        return cursor0, cursor1, ctxt

    @property
    def prices(self) -> Tuple[int, int]:
        return gpt_prices(self.model_name)

    @property
    def model_name(self) -> str:
        if not self.__model_name:
            model_name = 'gpt-3.5-turbo-0613'
            if self._model_n == 'gpt-3.5-turbo' or self._model_n == 'gpt-4':
                model_name = self._model_n + '-0613'
            self.__model_name = model_name
        return self.__model_name

    @model_name.setter
    def model_name(self, val: str):
        self.__model_name = val

    async def completion(self) -> Iterator[Dict[str, str]]:
        if self.max_tokens < 1: self.max_tokens = 256
        self.messages = self._messages()
        self.completion_so_far: str = ""
        self.metering_prompt_tokens_n = 0
        self.metering_generated_tokens_n = 0
        self.openai_prompt_tokens_n = 0
        self.openai_completion_tokens = 0

        def forward_streaming():
            modified = self._postprocess(self.completion_so_far)
            return {self.cursor_file: modified}

        try:
            gen = await openai.ChatCompletion.acreate(
                model=self.model_name,
                messages=self.messages,
                max_tokens=self.max_tokens,
                stream=self.stream,
                temperature=self.temp,
                stop=['<|end|>'],
            )

            if not self.stream:
                resp = gen
                self.completion_so_far = resp["choices"][0]["message"]["content"]
                if DEBUG:
                    sys.stdout.write(termcolor.colored(self.completion_so_far, "green"))
                    sys.stdout.flush()
                self.openai_prompt_tokens_n = resp["usage"]["prompt_tokens"]
                self.openai_completion_tokens = resp["usage"]["completion_tokens"]
                print(resp["usage"])
                self.model_name = resp["model"]
                self.finish_reason = resp["choices"][0]["finish_reason"] or "END"
            else:
                self.finish_reason = ""
                self.completion_so_far = ""
                tokens = 0
                while True:
                    resp = await asyncio.wait_for(gen.__anext__(), self._stream_timeout_sec)
                    delta = resp.choices[0].delta
                    if "content" in delta:
                        if DEBUG:
                            sys.stdout.write(termcolor.colored(delta["content"], "green"))
                            sys.stdout.flush()
                        self.completion_so_far += delta["content"]
                        tokens += 1  # assuming 1 token per chunk
                    if "model" in resp:
                        self.model_name = resp["model"]
                    if "finish_reason" in resp.choices[0] and resp.choices[0]["finish_reason"] is not None:
                        self.finish_reason = resp.choices[0]["finish_reason"]
                    if self.finish_reason:
                        break
                    if tokens % ACCUMULATE_N_STREAMING_CHUNKS == 0:
                        yield forward_streaming()
                    if self.finish_reason:
                        break
            if self.model_name == "":
                self.debuglog("ScratchpadChatGPT: model_name is empty")
            if self.finish_reason == "":
                self.finish_reason = "END"
        except asyncio.exceptions.TimeoutError as e:
            self.debuglog("FUNCTIONS TIMEOUT:", str(type(e)), str(e))
        except Exception as e:
            self.debuglog("FUNCTIONS EXCEPTION:", str(type(e)), str(e))
            self.finish_reason = "ERROR"
        yield forward_streaming()

    def _messages(self) -> List[Dict[str, str]]:
        raise NotImplementedError

    def _postprocess(self, completion: str) -> str:
        completion = code_block_postprocess(completion)
        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]

    def toplevel_fields(self):
        if not self.finish_reason:
            return {}
        else:
            calc_prompt_tokens_n, calc_generated_tokens_n = calculate_chat_tokens(
                self.model_name, self.messages, self.completion_so_far
            )
            self.metering_prompt_tokens_n = self.openai_prompt_tokens_n or calc_prompt_tokens_n
            self.metering_generated_tokens_n = self.openai_completion_tokens or calc_generated_tokens_n
            metering_message = {
                "metering_prompt_tokens_n": self.metering_prompt_tokens_n,
                "metering_generated_tokens_n": self.metering_generated_tokens_n,
                "pp1000t_prompt": self.prices[0],
                "pp1000t_generated": self.prices[1],
                "model_name": self.model_name,
            }
            self.debuglog(json.dumps(metering_message))
            return metering_message

    def debuglog(self, *args):
        if self._logger:
            self._logger(*args)
