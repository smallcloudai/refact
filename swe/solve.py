import json
import asyncio
import traceback
import jsonlines

from pathlib import Path
from more_itertools import chunked
from datasets import load_dataset


async def process_instance(instance_id: str, output: Path, timeout: int = 120):
    try:
        process = await asyncio.create_subprocess_exec(
            "python", "swe/swe_steps.py", instance_id,
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


async def main():
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--workers", type=int, default=1)
    parser.add_argument("--run", type=str, default="gpt35-gpt4")
    args = parser.parse_args()

    output = Path(__file__).parent / "predictions" / args.run
    for row_batch in chunked(load_dataset('princeton-nlp/SWE-bench_Lite', split='test'), n=args.workers):
        await asyncio.gather(*[process_instance(row["instance_id"], output) for row in row_batch])

    with jsonlines.open(output / "all_preds.jsonl", 'w') as f:
        # prds.append({"model_patch": "", **d})
        f.write_all([
            json.loads(f.read_text())
            for f in output.glob("*.json")
        ])


if __name__ == "__main__":
    asyncio.run(main())
