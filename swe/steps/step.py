from refact.chat_client import Message
from refact.chat_client import tools_fetch_and_filter
from refact.chat_client import ask_using_http
from refact.chat_client import print_messages

from typing import Set, Any, List, Iterable


__all__ = ["Step"]


class Step:
    def __init__(
            self,
            base_url: str,
            model_name: str,
            temperature: float = 0.2,
            max_depth: int = 8):
        self._base_url = base_url
        self._model_name = model_name
        self._temperature = temperature
        self._max_depth = max_depth

    @property
    def _tools(self) -> Set[str]:
        raise NotImplementedError()

    async def _query(self, messages: List[Message], stream: bool = False) -> List[Message]:
        tools = await tools_fetch_and_filter(
            base_url=self._base_url,
            tools_turn_on=self._tools)
        assistant_choices = await ask_using_http(
            self._base_url, messages, 1, self._model_name,
            tools=tools, verbose=False, temperature=self._temperature,
            stream=stream, max_tokens=2048,
            only_deterministic_messages=False,
        )
        new_messages = assistant_choices[0]
        print_messages(new_messages[len(messages):])
        return new_messages

    async def _query_generator(self, messages: List[Message], n: int) -> Iterable[List[Message]]:
        # tools = await tools_fetch_and_filter(
        #     base_url=self._base_url,
        #     tools_turn_on=self._tools)
        # assistant_choices = await ask_using_http(
        #     self._base_url, messages, n, self._model_name,
        #     tools=tools, verbose=True, temperature=self._temperature,
        #     stream=False, max_tokens=2048,
        #     only_deterministic_messages=False,
        # )
        # return assistant_choices
        raise NotImplementedError()

    async def process(self, **kwargs) -> Any:
        raise NotImplementedError()
