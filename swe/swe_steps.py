import json
import asyncio
import shutil
import subprocess

from uuid import uuid4
from datetime import datetime

from datasets import load_dataset
from argparse import ArgumentParser
from pathlib import Path
from typing import Dict, Any, List
from refact import chat_client

from lsp_runner import LSPServerRunner


REPOS_WORKDIR = Path("swe/repos")
DUMP_PREFIX = datetime.now().strftime("%Y%m%d-%H%M%S")

MODEL = "gpt-3.5-turbo"
# MODEL = "gpt-4o"

step1_tools_turn_on = {
    "file",
    "definition",
    "references",
}

task_message_marker = "=====TASK====="
done_message = "DONE"
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

step2_system_message = f"""
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
3. When you are done with the task, send the message including only one word: {done_message}
 
Changing tests is not allowed!

Explain your plan briefly before calling the tools in parallel.
IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST! USE TOOLS IN PARALLEL!
"""


def patch_generate(repo_name: Path, diffs: List[Dict[str, Any]]):
    for d in diffs:
        filename = repo_name / d["file_name"]
        if not filename.exists():
            raise RuntimeError(f"file {filename} doesn't exist\n\n{d}")
        text = filename.read_text()
        p0 = d["lines_remove"]
        p1 = d["lines_add"]
        if p0 not in text:
            raise RuntimeError(
                f"can't apply diff, there is no 'lines_remove' in text\n\n"
                f"{filename}:\n\n{text}\n\n"
                f"lines_remove:\n\n{p0}\n\n"
            )
        patched_text = text[:text.find(p0)] + p1 + text[text.find(p0) + len(p0):]
        filename.write_text(patched_text)
    result = subprocess.check_output(["git", "diff"], cwd=str(repo_name))
    return result.decode()


class RepoContext:

    def __init__(self, repo: str, base_commit: str, workdir: Path):
        self._workdir = workdir
        self._repo_name = repo
        self._base_commit = base_commit
        self._context_repo_path = workdir / str(uuid4())
        self._workdir.mkdir(exist_ok=True, parents=True)

    async def __aenter__(self):
        repo_path = self._workdir / self._repo_name.split("/")[-1]
        if not repo_path.exists():
            subprocess.call(["git", "clone", f"git@github.com:{self._repo_name}.git"], cwd=self._workdir)
        assert repo_path.exists()
        assert not self._context_repo_path.exists()
        subprocess.call(["cp", "-r", str(repo_path), str(self._context_repo_path)])
        subprocess.call(["git", "clean", "-fd"], cwd=str(self._context_repo_path))
        subprocess.call(["git", "reset", "--hard", self._base_commit], cwd=str(self._context_repo_path))
        subprocess.call(["git", "log", "-1"], cwd=str(self._context_repo_path))
        return self._context_repo_path

    async def __aexit__(self, exc_type, exc, tb):
        if self._context_repo_path.exists():
            shutil.rmtree(str(self._context_repo_path))


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
                return patch_generate(repo_path.absolute(), formatted_diff)
            except Exception as e:
                print(f"{e}: {m.content}")
                continue
        if messages[-1].role == "assistant" and messages[-1].content and done_message == messages[-1].content:
            break

    raise RuntimeError("can't solve the problem")


async def main():
    parser = ArgumentParser()
    parser.add_argument("instance_id", type=str, help="SWE instance id")
    parser.add_argument("--port", type=int, default=8110, help="refact lsp port")
    parser.add_argument("--output-dir", type=Path, default="swe/predictions/test", help="output directory")
    args = parser.parse_args()

    args.output_dir.mkdir(exist_ok=True, parents=True)
    output_filename = args.output_dir / f"{args.instance_id}.json"
    assert not output_filename.exists()

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

    base_url = f"http://127.0.0.1:{args.port}/v1"
    instance = swebench[args.instance_id]

    results = {
        "model_name_or_path": "refact-dev-0.1",
        "instance_id": args.instance_id,
        "problem_statement": instance["problem_statement"],
    }
    async with RepoContext(instance["repo"], instance["base_commit"], REPOS_WORKDIR) as repo_path:
        async with LSPServerRunner(port=args.port, repo_path=str(repo_path)):
            try:
                summarized_problem_statement = await step1(instance["problem_statement"], base_url=base_url)
                results["summarized_problem_statement"] = summarized_problem_statement
            except Exception as e:
                results["error"] = str(e)
            if "error" not in results:
                try:
                    results["model_patch"] = await step2(
                        summarized_problem_statement, base_url=base_url, repo_path=repo_path)
                except Exception as e:
                    results["error"] = str(e)
                    results["model_patch"] = ""

    with open(output_filename, "w") as f:
        json.dump(results, f)

    return results


if __name__ == "__main__":
    asyncio.run(main())
