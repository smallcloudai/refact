import json
import asyncio
import traceback

from argparse import ArgumentParser

from agent_runner import AgentRunner
from agent_runner import get_swe_bench_lite_instance
from step1 import ExploreRepoStep
from step2 import ProducePatchStep
from step3 import ChooseSolutionStep

from pathlib import Path
from typing import Dict, Any


# MODEL = "gpt-3.5-turbo"
MODEL = "gpt-4o"


# TODO: more logging for each step:
#  messages
#  compute cost
#  number of steps
#  step1 additional info about patched file
#  ...
class SWERunner(AgentRunner):
    async def _steps(self, base_url: str, repo_path: Path, *args, **kwargs) -> Dict[str, Any]:
        results: Dict[str, Any] = dict()
        problem_statement = kwargs["problem_statement"]

        # step1: explore repo, find files that can be useful for the problem
        step1 = ExploreRepoStep(base_url=base_url, model_name=MODEL, attempts=3)
        try:
            results["filenames_list_all"] = await step1.process(
                problem_statement=problem_statement,
                repo_path=repo_path)
            results["filenames_list"] = "\n".join(filter(
                lambda x: "test" not in x,
                results["filenames_list_all"].split("\n")
            ))
        except Exception as e:
            results["error"] = f"step1: {type(e)} {str(e) or traceback.format_exc()}"
            return results

        # step2: produce patches for the problem with given files from step1
        step2 = ProducePatchStep(base_url=base_url, model_name=MODEL, temperature=0.3, attempts=3)
        try:
            results["task"] = problem_statement
            if results["filenames_list"]:
                results["task"] = "\n\n".join([
                    results["task"],
                    f"Use these files to solve the problem:",
                    results["filenames_list"],
                ])
            results["model_patches"] = await step2.process(
                task=results["task"],
                repo_path=repo_path)
        except Exception as e:
            results["error"] = f"step2: {type(e)} {str(e) or traceback.format_exc()}"
            return results

        # step3: choose the best solution from the list of patches
        step3 = ChooseSolutionStep(base_url=base_url, model_name=MODEL)
        try:
            results["model_patch"] = await step3.process(
                problem_statement=problem_statement,
                model_patches=results["model_patches"],
                repo_path=repo_path)
        except Exception as e:
            results["error"] = f"step3: {type(e)} {str(e) or traceback.format_exc()}"
            return results

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
