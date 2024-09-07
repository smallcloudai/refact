import re

from refact import chat_client
from refact.chat_client import print_messages
from swe.steps import Step

from pathlib import Path
from typing import Set, List


SYSTEM_MESSAGE = """
You're Refact Dev a prefect AI assistant.

You plan is to:
- Look through the user's problem statement.
- Call tree tool to obtain repository structure.
- Provide a list of files that one would need to edit to fix the problem.

Please only provide the full path and return at least 5 files.
The returned files should be separated by new lines ordered by most to least important and wrapped with ```
For example:
```
file1.py
file2.py
```
"""


class ExploreRepoStep(Step):

    def __init__(self, attempts: int, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._attempts = attempts

    @staticmethod
    def _extract_filenames(text: str, repo_root, filter_tests: bool = False) -> List[str]:
        pattern = r'\b(?:[a-zA-Z]:\\|/)?(?:[\w-]+[/\\])*[\w-]+\.\w+\b'
        filenames = set([
            filename.replace(repo_root.lstrip("/"), "").lstrip("/")
            for filename in re.findall(pattern, text)
        ])
        if filter_tests:
            filenames = {f for f in filenames if "test" not in f.lower()}
        return list(filenames)

    @property
    def _tools(self) -> Set[str]:
        return set()

    async def process(self, problem_statement: str, repo_path: Path, **kwargs) -> List[str]:
        tree_tool_call_dict = chat_client.ToolCallDict(
            id=chat_client.gen_function_call_id(),
            function=chat_client.FunctionDict(arguments='{}', name='tree'),
            type='function')
        messages = [
            chat_client.Message(role="system", content=SYSTEM_MESSAGE),
            chat_client.Message(role="user", content=f"Problem statement:\n\n{problem_statement}"),
            chat_client.Message(role="assistant", finish_reason="tool_calls", tool_calls=[tree_tool_call_dict]),
        ]
        self._trajectory.extend(print_messages(messages))

        new_messages = await self._query(messages)
        self._trajectory.extend(print_messages(new_messages))

        res_message = new_messages[-1]
        if res_message.role != "assistant":
            raise RuntimeError(f"unexpected message role '{res_message.role}' for answer")
        if not isinstance(res_message.content, str):
            raise RuntimeError(f"unexpected content type '{type(res_message.content)}' for answer")
        found_files = self._extract_filenames(res_message.content, str(repo_path))
        if len(found_files) == 0:
            raise RuntimeError(f"no files found")
        return found_files

    # TODO: fix choices first
    # async def process(self, problem_statement: str, repo_path: Path, **kwargs) -> str:
    #     messages = [
    #         chat_client.Message(role="system", content=SYSTEM_MESSAGE),
    #         chat_client.Message(role="user", content=f"Problem statement:\n\n{problem_statement}"),
    #     ]
    #     # tool call query
    #     messages = await self._query(messages)
    #     # answer query
    #     results = []
    #     for idx, choices in enumerate(await self._query_generator(messages, self._attempts)):
    #         print("=" * 40 + f"ATT {idx + 1}" + "=" * 40)
    #         res_message = choices[-1]
    #         if res_message.role != "assistant" or not isinstance(res_message.content, str):
    #             print(res_message.role, type(res_message.content))
    #         else:
    #             print(res_message.content)
    #         print("-" * 85)
    #         # TODO: postprocess content
    #         results.append(res_message.content)
    #     return ""
