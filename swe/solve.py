import json
import asyncio
import traceback

from pathlib import Path
from more_itertools import chunked
from datasets import load_dataset


async def process_instance(instance_id: str, timeout: int = 120):
    try:
        process = await asyncio.create_subprocess_exec(
            "python", "swe/swe_steps.py", instance_id,
            "--timeout", str(timeout),
            "--output", "swe/predictions/gpt35-gpt4",
        )
        await process.communicate()
        print(f"successfully processed instance {instance_id}")
    except Exception as e:
        with open(Path("swe/predictions/gpt35-gpt4") / f"{instance_id}.json", "w") as f:
            json.dump({
                "instance_id": instance_id,
                "error": str(e) or traceback.format_exc(),
            }, f, indent=4)
        print(f"failed to process instance {instance_id}")


async def main():
    for row_batch in chunked(load_dataset('princeton-nlp/SWE-bench_Lite', split='test'), n=1):
        await asyncio.gather(*[process_instance(row["instance_id"]) for row in row_batch])


if __name__ == "__main__":
    asyncio.run(main())
