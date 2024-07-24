import random

from swe.steps import Step
from refact import chat_client
from refact.chat_client import print_block
from refact.chat_client import print_messages

from pathlib import Path
from typing import List, Set


DONE_MESSAGE = "=====PATCH====="
SYSTEM_MESSAGE = f"""
You are Refact Dev, an auto coding assistant.

You'll receive a problem statement with several solutions.
Your aim is to choose one solution that more accurately solves the problem.
Use tools to get access to the codebase. Use each tool exact in it's format do not add any extra args.

A good strategy to solve the issue is:
1. Build context. Before you move to the next step, make sure you collect all needed context: file names, code, etc.
2. Speculate about given solutions and choose the best one. Your last message should start with {DONE_MESSAGE} mark and should contain solution name only (for example Solution 99).

Explain your plan briefly before calling the tools in parallel.
IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST! USE TOOLS IN PARALLEL!
"""


class ChooseSolutionStep(Step):
    @property
    def _tools(self) -> Set[str]:
        return set()

    async def process(
            self,
            problem_statement: str,
            related_files: List[str],
            model_patches: List[str],
            repo_path: Path,
            **kwargs) -> str:
        if not model_patches:
            raise RuntimeError("no patches for problem")
        if len(model_patches) < 2:
            return model_patches[0]

        user_message_parts = [
            "Problem statement:",
            problem_statement,
        ]
        random.shuffle(model_patches)
        for idx, model_patch in enumerate(model_patches, start=1):
            user_message_parts.extend([
                f"Solution {idx}:",
                model_patch,
            ])

        paths = ",".join([str(repo_path / filename) for filename in related_files])
        files_tool_call_dict = chat_client.ToolCallDict(
            id=chat_client.gen_function_call_id(),
            function=chat_client.FunctionDict(arguments='{"paths":"' + paths + '"}', name='files_skeleton'),
            type='function')
        messages = [
            chat_client.Message(role="system", content=SYSTEM_MESSAGE),
            chat_client.Message(role="user", content="\n\n".join(user_message_parts)),
            chat_client.Message(role="assistant", finish_reason="tool_calls", tool_calls=[files_tool_call_dict]),
        ]
        self._trajectory.extend(print_messages(messages))

        # NOTE: 1 step should be enough to solve the problem
        for idx in range(self._max_depth):
            self._trajectory.append(print_block("iteration", idx + 1))

            new_messages = await self._query(messages)
            self._trajectory.extend(print_messages(new_messages))

            last_message = messages[-1]
            if last_message.role == "assistant" \
                    and last_message.content \
                    and DONE_MESSAGE in last_message.content:
                result = messages[-1].content.split(DONE_MESSAGE)[1].strip()
                for idx, model_patch in enumerate(model_patches, start=1):
                    if str(idx) in result:
                        return model_patch
                raise RuntimeError("can't choose a solution")
            messages.extend(new_messages)

        raise RuntimeError(f"can't solve the problem with {self._max_depth} iterations")
