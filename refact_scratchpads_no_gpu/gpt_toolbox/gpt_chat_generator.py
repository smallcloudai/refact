from typing import List, Dict

import openai


class ChatGenerator:
    def __init__(
            self,
            **kwargs
    ):
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
