import json
import asyncio
import difflib
import subprocess
from datetime import datetime

import termcolor
from datasets import load_dataset
from argparse import ArgumentParser
from pathlib import Path
from typing import Dict, Any, List
from refact import chat_client

from lsp_runner import LSPServerRunner


REPOS_WORKDIR = Path("swe-repos")
DUMP_PREFIX = datetime.now().strftime("%Y%m%d-%H%M%S")

MODEL = "gpt-3.5-turbo"
# MODEL = "gpt-4o"

step1_tools_turn_on = {
    "file",
    "definition",
    "references",
}

task_message_marker = "======TASK======"
step1_system_message = f"""
You are Refact Dev, an auto coding assistant.

You'll receive a problem statement from user.
Your aim is to rewrite it as a task for developer.

Use the following strategy:
1. Read the problem statement carefully.
2. Use given tools to explore code related to the issue and discuss how to solve it: you must find the real cause of the problem.
3. Set a task for developer that doesn't contain redundant information from the problem statement. Task should be started from {task_message_marker}.

Your final answer should be in the following format:
{task_message_marker}
todo explanation
task-related filenames list

Changing tests is not allowed!
Do not try to solve the issue yourself.
Your task must contain list of files that should be changed in the process of solving.
Each file name should contain full path to the file within the repo.

Explain your plan briefly before calling the tools in parallel.
IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST! USE TOOLS IN PARALLEL!
"""

step2_tools_turn_on = {
    "file",
    "definition",
    "patch",
}

step2_system_message = """
You are Refact Dev, an auto coding assistant.

You'll receive a problem statement from user.
Your aim is to solve this problem using speculation over the code and analyzing outputs of given tools.
Use tools to get access to the codebase. Use each tool exact in it's format do not add any extra args.

A good strategy to solve the issue is:
1. Build context:
 - use file, definition or references tools
 - before you move to the next step, make sure you collect all needed context: file names, code, etc.
2. Speculate about the problem and solve it:
 - describe what changes you need to do
 - apply changes to files separately using patch tool
 - path argument should be always full path to given file within repo
 - do not generate the patch itself, use patch(path, todo) tool to make this changes 
 
Changing tests is not allowed!

Explain your plan briefly before calling the tools in parallel.
IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST! USE TOOLS IN PARALLEL!
"""


def interactive_apply(repo_name: Path, diffs: List[Dict[str, Any]]):
    for d in diffs:
        filename = repo_name / d["file_name"]
        if not filename.exists():
            raise RuntimeError(f"file {filename} doesn't exist\n\n{d}")
        p0 = d["lines_remove"]
        p1 = d["lines_add"]
        text = filename.read_text()
        if p0 not in text:
            raise RuntimeError(
                f"can't apply diff, there is no 'lines_remove' in text\n\n"
                f"{filename}:\n\n{text}\n\n"
                f"lines_remove:\n\n{p0}\n\n"
            )
        text_patched = text[:text.find(p0)] + p1 + text[text.find(p0) + len(p0):]
        udiff = "\n".join(
            difflib.unified_diff(
                text.splitlines(), text_patched.splitlines(),
                lineterm="", n=0
            )
        )
        print(f"Refact generated the following patch to {filename}:\n\n{udiff}\n")
        while True:
            ans = input("Do you accept this patch? (y/n)")
            if ans == "y":
                filename.write_text(text_patched)
                print("patch applied")
            if ans not in ["y", "n"]:
                print("please answer y or n")
            else:
                break


def prepare_repo(repo: str, base_commit: str) -> Path:
    REPOS_WORKDIR.mkdir(exist_ok=True, parents=True)
    if not (REPOS_WORKDIR / repo).exists():
        subprocess.call(["git", "clone", f"git@github.com:{repo}.git"], cwd=REPOS_WORKDIR)
    repo_name = REPOS_WORKDIR / repo.split("/")[-1]
    assert repo_name.exists()
    subprocess.call(["git", "clean", "-fd"], cwd=repo_name)
    subprocess.call(["git", "reset", "--hard", base_commit], cwd=repo_name)
    subprocess.call(["git", "log", "-1"], cwd=repo_name)
    return repo_name


async def step1(problem_statement: str, base_url: str, steps: int = 10) -> str:
    messages = [
        chat_client.Message(role="system", content=step1_system_message),
        chat_client.Message(role="user", content=problem_statement),
    ]

    for step_n in range(steps):
        print(f"{'-' * 40} step {step_n} {'-' * 40}")
        tools = await chat_client.tools_fetch_and_filter(
            base_url=base_url,
            tools_turn_on=step1_tools_turn_on)
        assistant_choices = await chat_client.ask_using_http(
            base_url, messages, 1, MODEL,
            tools=tools, verbose=True, temperature=0.2,
            stream=False, max_tokens=2048,
            only_deterministic_messages=False,
        )

        messages = assistant_choices[0]
        if messages[-1].role == "assistant" and messages[-1].content and task_message_marker in messages[-1].content:
            return messages[-1].content.replace(task_message_marker, "")

    raise RuntimeError("can't find summarized context")


async def step2(summarized_problem_statement: str, base_url: str, repo_path: Path, steps: int = 10):
    messages = [
        chat_client.Message(role="system", content=step2_system_message),
        chat_client.Message(role="user", content=summarized_problem_statement),
    ]

    for step_n in range(steps):
        print(f"{'-' * 40} step {step_n} {'-' * 40}")
        tools = await chat_client.tools_fetch_and_filter(
            base_url=base_url,
            tools_turn_on=step2_tools_turn_on)
        assistant_choices = await chat_client.ask_using_http(
            base_url, messages, 1, MODEL,
            tools=tools, verbose=True, temperature=0.2,
            stream=False, max_tokens=2048,
            only_deterministic_messages=False,
        )

        messages = assistant_choices[0]

        applied_diff_call_ids = set()
        for m in [m for m in messages if m.role == "diff" and m.tool_call_id not in applied_diff_call_ids]:
            applied_diff_call_ids.add(m.tool_call_id)
            try:
                formatted_diff = json.loads(m.content)
                interactive_apply(repo_path, formatted_diff)
                input("Press Enter to continue generation or Ctrl+C to exit")
            except Exception as e:
                print(f"{e}: {m.content}")
                continue

    raise RuntimeError("can't solve the problem")


async def main():
    parser = ArgumentParser()
    parser.add_argument("instance_id", type=str, help="SWE instance id")
    args = parser.parse_args()

    swebench = {
        row["instance_id"]: {
            "repo": row["repo"],
            "base_commit": row["base_commit"],
            "problem_statement": row["problem_statement"],
            "patch": row["patch"],
        }
        for row in load_dataset('princeton-nlp/SWE-bench_Lite', split='test')
    }

    assert args.instance_id in swebench

    port = 8110
    base_url = f"http://127.0.0.1:{port}/v1"
    instance = swebench[args.instance_id]
    repo_path = prepare_repo(instance["repo"], instance["base_commit"])

    async with LSPServerRunner(port=port, repo_path=str(repo_path)):
        summarized_problem_statement = await step1(instance["problem_statement"], base_url=base_url)
        print(f"{termcolor.colored('Original problem statement:', 'red')}\n\n"
              f"{instance['problem_statement']}\n\n"
              f"{termcolor.colored('Summarized problem statement:', 'red')}\n\n"
              f"{summarized_problem_statement}\n\n")
        input("Press Enter to run the second step")
        await step2(summarized_problem_statement, base_url=base_url, repo_path=repo_path)


if __name__ == "__main__":
    asyncio.run(main())
