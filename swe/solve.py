import json
import asyncio
import traceback
import jsonlines

from pathlib import Path
from more_itertools import chunked
from datasets import load_dataset

from swe import SWE_WORKDIR


async def process_instance(instance_id: str, output: Path, timeout: int = 120):
    try:
        process = await asyncio.create_subprocess_exec(
            "python", "swe/utils/swe_steps.py", instance_id,
            "--timeout", str(timeout),
            "--output", str(output),
        )
        await process.communicate()
        print(f"successfully processed instance {instance_id}")
    except Exception as e:
        with open(output / f"{instance_id}.json", "w") as f:
            json.dump({
                "instance_id": instance_id,
                "error": str(e) or traceback.format_exc(),
            }, f, indent=4)
        print(f"failed to process instance {instance_id}")


def checkpoint_preds(output: Path):
    preds = [
        {"model_patch": "", **json.loads(f.read_text())}
        for f in output.glob("*.json")
    ]
    preds_filename = output / "all_preds.jsonl"
    with jsonlines.open(preds_filename, 'w') as f:
        f.write_all(preds)

    stats = {
        "instances": len(preds),
        "patched": len([p for p in preds if p["model_patch"]]),
        "prompt_tokens": 0,
        "completion_tokens": 0,
    }
    for p in preds:
        usages = p.get("usages", p.get("usage", {}))
        if "total_tokens" in usages:
            usages = {"step": usages}
        for usage in usages.values():
            stats["prompt_tokens"] += usage["prompt_tokens"]
            stats["completion_tokens"] += usage["completion_tokens"]

    stats_filename = output / "all_stats.txt"
    with open(stats_filename, "w") as f:
        for k, v in stats.items():
            f.write(f"{k:<20}{v:>10}\n")


async def main():
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--run", type=str, required=True)
    parser.add_argument("--workers", type=int, default=1)
    args = parser.parse_args()

    output = SWE_WORKDIR / "predictions" / args.run
    for row_batch in chunked(load_dataset('princeton-nlp/SWE-bench_Lite', split='test'), n=args.workers):
        await asyncio.gather(*[process_instance(row["instance_id"], output) for row in row_batch])
        checkpoint_preds(output)


if __name__ == "__main__":
    asyncio.run(main())
