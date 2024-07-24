import re
import json
import subprocess

from refact import chat_client
from step import Step

from pathlib import Path
from typing import List, Set


SYSTEM_MESSAGE = """
You're Refact Dev a prefect AI assistant.

You plan is to:
- Look through the user's problem statement and files structure.
- If needed collect context using definition and references tools.
- Call patch tool to produce a patch that solves given problem.

Rules of patch tool using:
- Choose exact one filename to patch.
- You should solve the problem with exact one patch tool call.

How patch tool's todo argument must looks like:
- Todo should contain the plan how to solve given problem with detailed description of each step.
- Add all needed symbols, their definitions and other code that should help with problem solving.
"""


class ProducePatchStep(Step):

    def __init__(self, attempts: int, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._attempts = attempts

    @property
    def _tools(self) -> Set[str]:
        return {
            "patch",
        }

    @staticmethod
    def _extract_filenames(text: str, filter_tests: bool = False) -> Set[str]:
        pattern = r'\b(?:[a-zA-Z]:\\|/)?(?:[\w-]+[/\\])*[\w-]+\.\w+\b'
        filenames = set(re.findall(pattern, text))
        if filter_tests:
            filenames = {f for f in filenames if "test" not in f.lower()}
        return filenames

    @staticmethod
    def _patch_tool_call_patch(message: chat_client.Message, problem_statement: str) -> chat_client.Message:
        if message.role != "assistant":
            raise RuntimeError("not an assistant message")
        if len(message.tool_calls) != 1 or message.tool_calls[0].function.name != "patch":
            raise RuntimeError("assistant must call exact one patch tool call")
        args = json.loads(message.tool_calls[0].function.arguments)
        if not args.get("todo", ""):
            raise RuntimeError("patch tool call must have todo argument")
        args["todo"] = "\n\n".join([
            "Problem statement:",
            problem_statement,
            "How you should solve the problem:",
            args["todo"],
        ])
        message.tool_calls[0].function.arguments = json.dumps(args)
        return message

    async def _patch_generate(self, message: chat_client.Message, repo_name: Path):
        if message.role != "diff":
            raise RuntimeError("not a diff message")
        formatted_diff = json.loads(message.content)
        await chat_client.diff_apply(self._base_url, chunks=formatted_diff, apply=[True] * len(formatted_diff))
        result = subprocess.check_output(["git", "--no-pager", "diff"], cwd=str(repo_name))
        await chat_client.diff_apply(self._base_url, chunks=formatted_diff, apply=[False] * len(formatted_diff))
        return result.decode()

    async def _attempt(self, messages: List[chat_client.Message], problem_statement: str, repo_name: Path) -> str:
        for _ in range(self._max_depth):
            new_messages = await self._query(messages)
            for idx in range(len(messages), len(new_messages)):
                try:
                    new_messages[idx] = self._patch_tool_call_patch(new_messages[idx], problem_statement)
                except:
                    pass
                try:
                    return await self._patch_generate(new_messages[idx], repo_name)
                except:
                    pass
            messages = new_messages
        raise RuntimeError(f"can't solve the problem with {self._max_depth} steps")

    async def process(self, problem_statement: str, related_files: str, repo_path: Path, **kwargs) -> List[str]:
        paths = ",".join([
            str(repo_path / filename)
            for filename in self._extract_filenames(related_files)
        ])
        files_tool_call_dict = chat_client.ToolCallDict(
            id=chat_client.gen_function_call_id(),
            function=chat_client.FunctionDict(arguments='{"paths":"' + paths + '"}', name='files_skeleton'),
            type='function')
        messages = [
            chat_client.Message(role="system", content=SYSTEM_MESSAGE),
            chat_client.Message(role="user", content=f"Problem statement:\n\n{problem_statement}"),
            chat_client.Message(role="assistant", finish_reason="tool_calls", tool_calls=[files_tool_call_dict]),
        ]

        results = []
        for _ in range(self._attempts):
            try:
                results.append(await self._attempt(messages, problem_statement, repo_path.absolute()))
            except RuntimeError:
                continue
        return results
