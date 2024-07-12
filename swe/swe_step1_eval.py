import json
import re
import asyncio
import traceback
import whatthepatch

from argparse import ArgumentParser

from agent_runner import AgentRunner
from agent_runner import get_swe_bench_lite_instance
from step1 import SetTaskStep

from pathlib import Path
from typing import Dict, Any


# MODEL = "gpt-3.5-turbo"
MODEL = "gpt-4o"


class SWERunner(AgentRunner):

    @staticmethod
    def _patched_file(patch: str) -> str:
        files = list(whatthepatch.parse_patch(patch))
        assert len(files) == 1
        header = files[0].header
        assert header.old_path[len("a/"):] == header.new_path[len("b/"):]
        return header.old_path[len("a/"):]

    @staticmethod
    def _filename_mentioned(filename: str, text: str) -> str:
        if filename in text:
            return "fully"
        elif Path(filename).name in text:
            return "partially"
        return "no"

    async def _steps(self, base_url: str, repo_path: Path, *args, **kwargs) -> Dict[str, Any]:
        results: Dict[str, Any] = dict()
        problem_statement = kwargs["problem_statement"]
        filename: str = self._patched_file(kwargs["problem_patch"])
        results["patched_filename"] = filename
        results["mentioned_in_problem"] = \
            self._filename_mentioned(filename, problem_statement)
        try:
            step1 = SetTaskStep(base_url=base_url, model_name=MODEL)
            results["summarized_problem_statement"] = \
                await step1.process(problem_statement=kwargs["problem_statement"], repo_path=repo_path)
            results["mentioned_in_task"] = \
                self._filename_mentioned(filename, results["summarized_problem_statement"])
        except Exception as e:
            raise RuntimeError(f"step1: {type(e)} {str(e) or traceback.format_exc()}")
        return results


async def main():
    parser = ArgumentParser()
    parser.add_argument("instance_id", type=str, help="SWE instance id")
    parser.add_argument("--timeout", type=float, default=None, help="processing timeout")
    parser.add_argument("--output-dir", type=Path, default=None, help="output directory")
    args = parser.parse_args()

    if args.output_dir is not None:
        args.output_dir.mkdir(exist_ok=True, parents=True)
        output_filename = args.output_dir / f"{args.instance_id}.json"
        if output_filename.exists():
            print(f"skip {args.instance_id} because it's already done")
            exit(0)

    instance = get_swe_bench_lite_instance(args.instance_id)
    run_postfix = f"-{args.output_dir.name}" if args.output_dir is not None else ""
    results = {
        "model_name_or_path": f"refact-dev-{MODEL}{run_postfix}",
        "instance_id": args.instance_id,
        "problem_statement": instance["problem_statement"],
        "problem_patch": instance["patch"],
    }

    try:
        runner = SWERunner(
            timeout=args.timeout)
        results.update(await runner.run(
            repo_name=instance["repo"],
            base_commit=instance["base_commit"],
            **results,
        ))
    except Exception as e:
        results["error"] = str(e) or traceback.format_exc()

    if args.output_dir is not None:
        with open(output_filename, "w") as f:
            json.dump(results, f, indent=4)
    else:
        print(json.dumps(results, indent=4))

    return results


if __name__ == "__main__":
    asyncio.run(main())
