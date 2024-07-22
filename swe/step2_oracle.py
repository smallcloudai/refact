import json
import subprocess

from step import Step
from refact import chat_client

from pathlib import Path
from typing import Dict, Any, List, Set


SYSTEM_MESSAGE = f"""
You are Refact Dev, an auto coding assistant.

You'll receive following info from user:
 - problem statement
 - exact one filename that should be patched

Pass filename without any changes as paths arg for patch tool.
Pass problem statement as todo arg for patch tool. Do not change anything in it.
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

    async def _patch_generate(self, repo_name: Path, formatted_diff: List[Dict[str, Any]]):
        await chat_client.diff_apply(self._base_url, chunks=formatted_diff, apply=[True] * len(formatted_diff))
        result = subprocess.check_output(["git", "--no-pager", "diff"], cwd=str(repo_name))
        await chat_client.diff_apply(self._base_url, chunks=formatted_diff, apply=[False] * len(formatted_diff))
        return result.decode()

    async def process(self, task: str, repo_path: Path, **kwargs) -> str:
        messages = [
            chat_client.Message(role="system", content=SYSTEM_MESSAGE),
            chat_client.Message(role="user", content=task),
        ]

        for step_n in range(self._max_depth):
            print(f"{'-' * 40} step {step_n} {'-' * 40}")
            messages = await self._query(messages)
            applied_diff_call_ids = set()
            for m in [m for m in messages if m.role == "diff" and m.tool_call_id not in applied_diff_call_ids]:
                applied_diff_call_ids.add(m.tool_call_id)
                try:
                    formatted_diff = json.loads(m.content)
                    return await self._patch_generate(repo_path.absolute(), formatted_diff)
                except json.decoder.JSONDecodeError:
                    continue
        raise RuntimeError(f"can't solve the problem with {self._max_depth} steps")
