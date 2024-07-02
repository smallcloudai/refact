import json
import asyncio
import traceback

from argparse import ArgumentParser

from agent_runner import AgentRunner
from agent_runner import get_swe_bench_lite_instance
from step1 import SetTaskStep
from step2 import SolveTaskStep

from pathlib import Path
from typing import Dict, Any


MODEL = "gpt-3.5-turbo"


class SWERunner(AgentRunner):
    async def _steps(self, base_url: str, repo_path: Path, *args, **kwargs) -> Dict[str, Any]:
        results: Dict[str, Any] = dict()
        try:
            step1 = SetTaskStep(base_url=base_url, model_name=MODEL)
            results["task"] = await step1.process(problem_statement=kwargs["problem_statement"])
        except Exception as e:
            raise RuntimeError(f"step1: {type(e)} {str(e) or traceback.format_exc()}")
        try:
            step2 = SolveTaskStep(base_url=base_url, model_name=MODEL)
            results["model_patch"] = await step2.process(task=results["task"], repo_path=repo_path)
        except Exception as e:
            raise RuntimeError(f"step2: {type(e)} {str(e) or traceback.format_exc()}")
        return results


async def main():
    parser = ArgumentParser()
    parser.add_argument("instance_id", type=str, help="SWE instance id")
    parser.add_argument("--timeout", type=float, default=None, help="processing timeout")
    parser.add_argument("--output-dir", type=Path, default="swe/predictions/test", help="output directory")
    args = parser.parse_args()

    args.output_dir.mkdir(exist_ok=True, parents=True)
    output_filename = args.output_dir / f"{args.instance_id}.json"
    if output_filename.exists():
        print(f"skip {args.instance_id} because it's already done")
        exit(0)

    instance = get_swe_bench_lite_instance(args.instance_id)
    results = {
        "model_name_or_path": "refact-dev-gpt35-gpt4",
        "instance_id": args.instance_id,
        "problem_statement": instance["problem_statement"],
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

    with open(output_filename, "w") as f:
        json.dump(results, f, indent=4)

    return results


if __name__ == "__main__":
    asyncio.run(main())
