import ujson as json
from refact import chat_client
from refact.chat_client import print_messages
from swe.steps import Step

from pathlib import Path
from typing import List, Dict, Any, Set


class RelevantFiles(Step):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    @property
    def _tools(self) -> Set[str]:
        return set()

    async def process(self, problem_statement: str, repo_path: Path, **kwargs) -> List[str]:
        tree_tool_call_dict = chat_client.ToolCallDict(
            id=chat_client.gen_function_call_id(),
            function=chat_client.FunctionDict(arguments='{}', name='relevant_files'),
            type='function')

        messages = [
            chat_client.Message(role="user", content=f"Problem statement:\n\n{problem_statement}"),
            chat_client.Message(role="assistant", finish_reason="tool_calls", tool_calls=[tree_tool_call_dict]),
        ]
        self._trajectory.extend(print_messages(messages))

        new_messages = await self._query(messages, only_deterministic_messages=True)
        self._trajectory.extend(print_messages(new_messages))

        res_message = [m for m in new_messages if m.role == "tool"][-1]
        try:
            files_list: List[str] = json.loads(res_message.content)
        except Exception as e:
            raise RuntimeError(f"content is not decodable as json:\n{res_message.content}\nError: {e}")

        if not files_list:
            raise RuntimeError(f"no files found")

        return files_list
