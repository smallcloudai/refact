import random

from step import Step
from refact import chat_client

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
        return {
            "file",
            "definition",
        }

    async def process(self, problem_statement: str, model_patches: List[str], repo_path: Path, **kwargs) -> str:
        if not model_patches:
            raise RuntimeError("no patches for problem")
        if len(model_patches) < 2:
            return model_patches[0]

        message_parts = [
            "Problem statement:",
            problem_statement,
        ]

        random.shuffle(model_patches)
        for idx, model_patch in enumerate(model_patches, start=1):
            message_parts.extend([
                f"Solution {idx}:",
                model_patch,
            ])

        messages = [
            chat_client.Message(role="system", content=SYSTEM_MESSAGE),
            chat_client.Message(role="user", content="\n\n".join(message_parts)),
        ]

        for step_n in range(self._max_depth):
            print(f"{'-' * 40} step {step_n} {'-' * 40}")
            tools = await chat_client.tools_fetch_and_filter(
                base_url=self._base_url,
                tools_turn_on=self._tools)
            assistant_choices = await chat_client.ask_using_http(
                self._base_url, messages, 1, self._model_name,
                tools=tools, verbose=True, temperature=self._temperature,
                stream=False, max_tokens=2048,
                only_deterministic_messages=False,
            )

            messages = assistant_choices[0]
            last_message = messages[-1]
            if last_message.role == "assistant" \
                    and last_message.content \
                    and DONE_MESSAGE in last_message.content:
                result = messages[-1].content.split(DONE_MESSAGE)[1].strip()
                for idx, model_patch in enumerate(model_patches, start=1):
                    if str(idx) in result:
                        return model_patch
                raise RuntimeError("can't choose a solution")

        raise RuntimeError(f"can't solve the problem with {self._max_depth} steps")
