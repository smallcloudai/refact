import json
import asyncio
import traceback
import whatthepatch

from argparse import ArgumentParser

from agent_runner import AgentRunner
from agent_runner import get_swe_bench_lite_instance
from step2_oracle import ProducePatchStep

from pathlib import Path
from typing import Dict, Any


MODEL = "gpt-4o"


def patched_file(patch: str) -> str:
    files = list(whatthepatch.parse_patch(patch))
    assert len(files) == 1
    header = files[0].header
    assert header.old_path[len("a/"):] == header.new_path[len("b/"):]
    return header.old_path[len("a/"):]


class SWERunner(AgentRunner):

    async def _steps(self, base_url: str, repo_path: Path, *args, **kwargs) -> Dict[str, Any]:
        results: Dict[str, Any] = dict()
        try:
            step = ProducePatchStep(base_url=base_url, model_name=MODEL, attempts=3)
            results["model_patch"] = \
                await step.process(task=kwargs["summarized_problem_statement"], repo_path=repo_path)
        except Exception as e:
            raise RuntimeError(f"step2: {type(e)} {str(e) or traceback.format_exc()}")
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
        results["summarized_problem_statement"] = "\n\n".join([
            "Problem statement:",
            results["problem_statement"],
            "File to patch:",
            patched_file(results["problem_patch"]),
        ])
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
