import asyncio
import json
import os

import functools

from typing import *

from refact_scratchpads_no_gpu.async_scratchpad import ascratch

import openai
import tiktoken

from refact_scratchpads_no_gpu.gpt_toolbox.smc_functions import SMC_FUNCTIONS
from refact_vecdb import VecDBAsyncAPI
from refact_scratchpads_no_gpu.gpt_toolbox.websearch import WebSearch

DEBUG = int(os.environ.get("DEBUG", "0"))


def gpt_prices(  # Apr 4 2023:
        model_name: str,
) -> Tuple[int, int]:
    # GPT-4 8K prompt[$0.03 / 1K tokens] generated[$0.06 / 1K tokens]
    if model_name.startswith("gpt-4") or model_name.startswith("gpt4"):
        pp1000t_prompt = 30_000
        pp1000t_generated = 60_000
    # gpt-3.5-turbo $0.002 / 1K tokens
    elif model_name.startswith("gpt-3.5-turbo"):
        pp1000t_prompt = 2_000
        pp1000t_generated = 2_000
    else:
        raise ValueError(f'get_prices: Unknown model: {model_name}')
    return pp1000t_prompt, pp1000t_generated


@functools.lru_cache(maxsize=10)
def engine_to_encoding(engine: str) -> tiktoken.Encoding:
    enc = tiktoken.encoding_for_model(engine)
    return enc


ACCUMULATE_N_STREAMING_CHUNKS = 5
engine_to_encoding("text-davinci-003")  # this immediately tests if tiktoken works or not


def calculate_chat_tokens(model_name, messages, completion):
    enc = engine_to_encoding(model_name)
    calc_prompt_tokens_n = 2  # warmup
    for d in messages:
        calc_prompt_tokens_n += len(enc.encode(d["content"], disallowed_special=()))
        calc_prompt_tokens_n += len(enc.encode(d["role"], disallowed_special=()))
        calc_prompt_tokens_n += 4  # to switch user/assistant
    calc_generated_tokens_n = len(enc.encode(completion, disallowed_special=())) + 2  # one to switch, another EOF
    return calc_prompt_tokens_n, calc_generated_tokens_n


class ChatGenerator:
    def __init__(
            self,
            use_functions: bool = True,
            **kwargs
    ):
        self._use_functions = use_functions
        if self._use_functions:
            kwargs = {
                **kwargs,
                'functions': SMC_FUNCTIONS,
                'function_call': 'auto'
            }
        self.kwargs = kwargs
        self.__gen = None

    async def __aiter__(self):
        await self._init()
        return self

    async def _init(self):
        if not self.__gen:
            self.__gen = await openai.ChatCompletion.acreate(
                stream=True,
                **self.kwargs
            )

    async def __anext__(self):
        await self._init()
        try:
            return await self.__gen.__anext__()
        except StopAsyncIteration:
            pass


class GptChatWithFunctions(ascratch.AsyncScratchpad):
    def __init__(
            self,
            id: str,
            *,
            created: float,
            temperature: float,
            top_p: float,
            max_tokens: int,
            stop_tokens: Union[str, List[str]],
            messages: List[Dict[str, str]],
            model: str,  # always "longthink", don't use
            **more,
    ):
        super().__init__(
            id=id,
            created=created,
            temperature=temperature,
            top_p=top_p,
            max_tokens=max_tokens,
            stop_tokens=stop_tokens,
            **more,
        )

        self._model_n = "gpt-3.5-turbo"
        if "gpt4" in self.function or "gpt-4" in self.function:
            self._model_n = "gpt-4"
        self.__model_name = None

        self._stream_timeout_sec = 15
        self.metering_prompt_tokens_n = 0
        self.metering_generated_tokens_n = 0

        messages = messages or []
        if not messages or messages[0].get('role') != 'system':
            messages = [
                {
                    "role": "system",
                    "content": "You are a coding assistant that outputs short answers, gives links to documentation.",
                }, *messages
            ]
        self._messages = messages
        self._completion = ""
        self._function_call = {}
        self._on_function = False

        self._vecdb = VecDBAsyncAPI()
        self._websearch = WebSearch()

    @property
    def _model_name(self) -> str:
        if not self.__model_name:
            model_name = 'gpt-3.5-turbo-0613'
            if self._model_n == 'gpt-3.5-turbo' or self._model_n == 'gpt-4':
                model_name = self._model_n + '-0613'
            self.__model_name = model_name
        return self.__model_name

    @_model_name.setter
    def _model_name(self, val: str):
        self.__model_name = val

    @property
    def prices(self) -> Tuple[int, int]:
        return gpt_prices(self._model_name)

    async def _resolve_function(self, name: str, params: Dict[str, Any]):
        if DEBUG:
            self.debuglog(f'CALLING function {name} with params {params}')
        if name == 'get_current_weather':
            return f'{name}({", ".join([v for k, v in params.items()])})'
        if name == 'get_code_examples':
            param = params.get('look_for', 'unknown')
            candidates = await self._vecdb.find(param, 1)
            text = candidates[0]['text']
            self._messages = [
                *self._messages,
                {
                    'role': 'user',
                    'content':
                        f'This an example of using {param}. I want you to understand how it is used:\n'
                        f'{text}\n'
                        f'Answer on two questions:\n'
                        f'1. What is the purpose of {param}\n'
                        f'2. Write a short example of usage {param} abstracting from the context'
                }
            ]
        if name == 'web_search':
            param = params.get('query', 'unknown')
            candidates = await self._websearch.a_search(param)
            search_result = json.dumps([{'link': c.link, 'snippet': c.snippet} for c in candidates])

            self._messages = [
                *self._messages,
                {
                    'role': 'user',
                    'content':
                        f'I wanted to find out about: {param} and performed a google search request. '
                        f'I have got top-{len(candidates)} results. I want you to summarize them very briefly. '
                        f'Output summarization with references to each source you found informative. '
                        f'Here are the results of my search: \n {search_result}'
                }
            ]

    def _create_chat_gen(self, use_functions: bool) -> ChatGenerator:
        return ChatGenerator(
            use_functions=use_functions,
            model=self._model_name,
            messages=self._messages,
            max_tokens=self.max_tokens,
            temperature=self.temp,
        )

    async def call_openai_api_based_on_stored_messages(self, use_functions: bool = True) -> AsyncIterator[Dict[str, str]]:
        if DEBUG:
            self.debuglog(f'MODEL_NAME={self._model_name}')
        self._function_call = {}
        self.finish_reason = ''
        self._completion = ''
        accum = ""
        role = ""
        tokens = 0

        gen = self._create_chat_gen(use_functions=use_functions)

        async def accumulator_to_a_streaming_packet(final_it: bool = False):
            nonlocal accum

            def msg(content: str) -> Dict[str, str]:
                return {
                    "chat__role": role,
                    "chat__content": content
                }

            if self._on_function and final_it:
                try:
                    self._function_call['arguments'] = json.loads(self._function_call['arguments'])
                except json.JSONDecodeError:
                    return msg('')

                await self._resolve_function(self._function_call['name'], self._function_call['arguments'])
                self._function_call.clear()
                return msg('')

            self._completion += accum
            accum = ""
            return msg(self._completion)

        try:
            while True:
                resp = await asyncio.wait_for(gen.__anext__(), self._stream_timeout_sec)
                if not resp:
                    break
                delta = resp.choices[0].delta
                if function_call := delta.get('function_call'):
                    self._on_function = True
                    self._function_call.setdefault('name', function_call.get('name'))
                    self._function_call.setdefault('arguments', '')
                    self._function_call['arguments'] += function_call['arguments']
                if "role" in delta:
                    role = delta["role"]
                if content := delta.get('content'):
                    accum += content
                    tokens += 1  # assuming 1 token per chunk
                if "finish_reason" in resp.choices[0] and \
                        resp.choices[0]["finish_reason"] is not None and \
                        not self._on_function:
                    self.finish_reason = resp.choices[0]["finish_reason"]
                if self.finish_reason:
                    break
                if (tokens % ACCUMULATE_N_STREAMING_CHUNKS == 0) and not self._on_function:
                    yield await accumulator_to_a_streaming_packet()
                if self.finish_reason:  # cancelled from main coroutine
                    break

        except asyncio.exceptions.TimeoutError as e:
            self.debuglog("CHAT TIMEOUT:", str(type(e)), str(e))
        except Exception as e:
            self.debuglog("CHAT EXCEPTION:", str(type(e)), str(e))
        yield await accumulator_to_a_streaming_packet(final_it=True)

    async def completion(self) -> AsyncIterator[Dict[str, str]]:
        async for res in self.call_openai_api_based_on_stored_messages():
            yield res

        if self._on_function:
            self._on_function = False
            async for res in self.call_openai_api_based_on_stored_messages(use_functions=False):
                yield res

        self.finish_reason = 'END'

    def toplevel_fields(self):
        if not self.finish_reason:
            return {}
        else:
            calc_prompt_tokens_n, calc_generated_tokens_n = calculate_chat_tokens(
                self._model_name, self._messages, self._completion
            )
            self.metering_prompt_tokens_n = calc_prompt_tokens_n
            self.metering_generated_tokens_n = calc_generated_tokens_n
            return {
                "metering_prompt_tokens_n": self.metering_prompt_tokens_n,
                "metering_generated_tokens_n": self.metering_generated_tokens_n,
                "pp1000t_prompt": self.prices[0],
                "pp1000t_generated": self.prices[1],
                "model_name": self._model_name,
            }
