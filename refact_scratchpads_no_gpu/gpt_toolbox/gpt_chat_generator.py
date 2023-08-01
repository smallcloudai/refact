from typing import List, Dict

import openai

from refact_scratchpads_no_gpu.gpt_toolbox.smc_functions import SMC_FUNCTIONS


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
