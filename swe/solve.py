import asyncio

from more_itertools import chunked
from datasets import load_dataset


async def process_instance(instance_id: str, port: int, timeout: int = 120):
    try:
        process = await asyncio.create_subprocess_exec(
            "python", "swe/swe_steps.py", instance_id,
            "--port", str(port),
            "--output", "swe/predictions/gpt35")
        await asyncio.wait_for(process.communicate(), timeout)
        print(f"successfully processed instance {instance_id}")
    except Exception as e:
        print(f"failed to process instance {instance_id}: {e}")


async def main():
    for row_batch in chunked(load_dataset('princeton-nlp/SWE-bench_Lite', split='test'), n=8):
        await asyncio.gather(*[
            process_instance(row["instance_id"], 8110 + idx)
            for idx, row in enumerate(row_batch)
        ])


if __name__ == "__main__":
    asyncio.run(main())
