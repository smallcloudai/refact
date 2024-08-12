import json
import asyncio
import traceback
import jsonlines
from tqdm import tqdm

from pathlib import Path
from more_itertools import chunked
from datasets import load_dataset


async def process_instance(instance_id: str, output_dir: Path, timeout: int = 120):
    cmdline = [
        "python",
        (Path(__file__).parent / "utils" / "step1_xentropy.py").as_posix(),
        "--timeout", str(timeout),
        "--output-dir", str(output_dir),
        instance_id,
    ]
    print(" ".join(cmdline))
    try:
        process = await asyncio.create_subprocess_exec(cmdline)
        await process.communicate()
        print(f"successfully processed instance {instance_id}")
    except Exception as e:
        if not (output_dir / f"{instance_id}.json").exists():
            with open(output_dir / f"{instance_id}.json", "w") as f:
                json.dump({
                    "instance_id": instance_id,
                    "error": str(e) or traceback.format_exc(),
                }, f, indent=4)
        print(f"failed to process instance {instance_id}")


def checkpoint_preds(output_dir: Path):
    preds = [
        {"model_patch": "", **json.loads(f.read_text())}
        for f in output_dir.glob("*.json")
    ]
    preds_filename = output_dir / "all_preds.jsonl"
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
            stats["prompt_tokens"] += usage.get("prompt_tokens", 0)
            stats["completion_tokens"] += usage.get("completion_tokens", 0)

    stats_filename = output_dir / "all_stats.txt"
    with open(stats_filename, "w") as f:
        for k, v in stats.items():
            f.write(f"{k:<20}{v:>10}\n")


async def main():
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=1)
    args = parser.parse_args()

    dataset = load_dataset('princeton-nlp/SWE-bench_Lite', split='test')
    dataset = list(dataset)
    dataset = dataset[:5]
    for row_batch in tqdm(chunked(dataset, n=args.workers), total=len(dataset) // args.workers):
        await asyncio.gather(*[process_instance(row["instance_id"], args.output_dir) for row in row_batch])
        checkpoint_preds(args.output_dir)


if __name__ == "__main__":
    asyncio.run(main())
