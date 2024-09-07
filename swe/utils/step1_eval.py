import json
import asyncio
import traceback

from argparse import ArgumentParser

from swe.utils import AgentRunner
from swe.utils import get_swe_bench_lite_instance
from swe.steps import ExploreRepoStep, Locate
from swe.utils.common import patched_file
from swe.utils.common import filename_mentioned

from pathlib import Path
from typing import Dict, Any, Tuple


# MODEL = "gpt-4o"
MODEL = "gpt-4o-mini"


class SWERunner(AgentRunner):

    async def _steps(self, base_url: str, repo_path: Path, *args, **kwargs) -> Tuple[Dict[str, Any], str]:
        results: Dict[str, Any] = dict()
        problem_statement = kwargs["problem_statement"]
        filename: str = patched_file(kwargs["problem_patch"])
        results["patched_file"] = filename
        results["patched_file_mentioned_in_problem"] = filename_mentioned(filename, problem_statement)
        step = Locate(base_url=base_url, model_name=MODEL, attempts=1)
        try:
            res = await step.process(
                problem_statement=problem_statement,
                repo_path=repo_path
            )
            results["found_files"] = res['context_files']
            results['to_change_files'] = res['to_change_files']
            results["patched_file_is_found"] = filename_mentioned(filename, "\n".join(results["found_files"]))
            results["to_change_file_is_found"] = filename_mentioned(filename, "\n".join(results["to_change_files"]))
        except Exception as e:
            raise e
            results["error"] = f"step1: {type(e)} {str(e) or traceback.format_exc()}"
        results["model_name"] = step.model_name
        results["usage"] = step.usage
        return results, step.trajectory


async def main():
    parser = ArgumentParser()
    parser.add_argument("instance_id", type=str, help="SWE instance id")
    parser.add_argument("--timeout", type=float, default=None, help="processing timeout")
    parser.add_argument("--output-dir", type=Path, default=None, help="output directory")
    args = parser.parse_args()

    if args.output_dir is not None:
        args.output_dir.mkdir(exist_ok=True, parents=True)
        result_filename = args.output_dir / f"{args.instance_id}.json"
        traj_filename = args.output_dir / f"{args.instance_id}.md"
        if result_filename.exists():
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
    traj = ""

    try:
        runner = SWERunner(
            timeout=args.timeout,
            use_ast=True,
            use_vecdb=False,
        )
        r, traj = await runner.run(
            repo_name=instance["repo"],
            base_commit=instance["base_commit"],
            **results,
        )
        results.update(**r, **results)
    except Exception as e:
        raise e
        results["error"] = str(e) or traceback.format_exc()

    if args.output_dir is not None:
        with open(result_filename, "w") as f:
            json.dump(results, f, indent=4)
        with open(traj_filename, "w") as f:
            f.write(traj)
    else:
        print(json.dumps(results, indent=4))

    return results


if __name__ == "__main__":
    asyncio.run(main())
