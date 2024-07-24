import json
import asyncio
import traceback

from argparse import ArgumentParser

from swe.utils import AgentRunner
from swe.utils import get_swe_bench_lite_instance
from swe.steps import ExploreRepoStep
from swe.steps import ProducePatchStep
from swe.steps import ChooseSolutionStep
from swe.utils.common import patched_file
from swe.utils.common import filename_mentioned

from pathlib import Path
from typing import Dict, Any


MODEL = "gpt-4o"


# TODO: more logging for each step:
#  messages
class SWERunner(AgentRunner):
    async def _steps(self, base_url: str, repo_path: Path, *args, **kwargs) -> Dict[str, Any]:
        results: Dict[str, Any] = {
            "model_name": MODEL,
            "usages": dict(),
        }
        problem_statement = kwargs["problem_statement"]
        filename = patched_file(kwargs["problem_patch"])

        # step1: explore repo, find files that can be useful for the problem
        results["patched_file"] = filename
        results["patched_file_mentioned_in_problem"] = filename_mentioned(filename, problem_statement)
        step1 = ExploreRepoStep(base_url=base_url, model_name=MODEL, attempts=3)
        try:
            results["found_files"] = await step1.process(
                problem_statement=problem_statement,
                repo_path=repo_path)
            results["patched_file_is_found"] = filename_mentioned(filename, "\n".join(results["found_files"]))
            results["usages"]["step1"] = step1.usage
        except Exception as e:
            results["error"] = f"step1: {type(e)} {str(e) or traceback.format_exc()}"
            results["usages"]["step1"] = step1.usage
            return results

        # step2: produce patches for the problem with given files from step1
        step2 = ProducePatchStep(base_url=base_url, model_name=MODEL, attempts=1)
        try:
            results["model_patches"] = await step2.process(
                problem_statement=problem_statement,
                related_files=results["found_files"],
                repo_path=repo_path)
            results["usages"]["step2"] = step2.usage
        except Exception as e:
            results["error"] = f"step2: {type(e)} {str(e) or traceback.format_exc()}"
            results["usages"]["step2"] = step2.usage
            return results

        # step3: choose the best solution from the list of patches
        step3 = ChooseSolutionStep(base_url=base_url, model_name=MODEL)
        try:
            results["model_patch"] = await step3.process(
                problem_statement=problem_statement,
                related_files=results["found_files"],
                model_patches=results["model_patches"],
                repo_path=repo_path)
            results["usages"]["step3"] = step3.usage
        except Exception as e:
            results["error"] = f"step3: {type(e)} {str(e) or traceback.format_exc()}"
            results["usages"]["step3"] = step3.usage
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
