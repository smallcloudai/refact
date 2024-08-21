import ujson as json
from refact import chat_client
from refact.chat_client import print_messages
from swe.steps import Step

from pathlib import Path
from typing import List, Dict, Any, Set


class Locate(Step):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    @property
    def _tools(self) -> Set[str]:
        return set()

    async def process(self, problem_statement: str, repo_path: Path, **kwargs) -> Dict[str, Any]:

        tool_args = {
            "problem_statement": problem_statement,
        }

        tool_call_dict = chat_client.ToolCallDict(
            id=chat_client.gen_function_call_id(),
            function=chat_client.FunctionDict(arguments=json.dumps(tool_args), name='locate'),
            type='function')

        messages = [
            chat_client.Message(role="assistant", finish_reason="tool_calls", tool_calls=[tool_call_dict]),
        ]
        self._trajectory.extend(print_messages(messages))

        new_messages = await self._query(messages, only_deterministic_messages=True)
        self._trajectory.extend(print_messages(new_messages))

        res_message = [m for m in new_messages if m.role == "tool"][-1]
        try:
            results: Dict[str, List[Any]] = json.loads(res_message.content)
        except Exception as e:
            raise RuntimeError(f"content is not decodable as json:\n{res_message.content}\nError: {e}")

        if 1:
            # Oleg branch
            # {
            #   "filename": {
            #     "SYMBOLS": "A,B,C",
            #     "WHY_CODE": "DEFINITIONS",
            #     "WHY_DESC": "Defines the index domain and its directives, crucial for understanding how index...",
            #     "RELEVANCY": 5
            #   },
            # }
            return results, []
        else:
            # Valerii branch
            files_list = results.get('files')
            symbols = results.get('symbols')
            return files_list, symbols
