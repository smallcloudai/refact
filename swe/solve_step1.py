import json
import asyncio
import traceback
from pathlib import Path
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
        process = await asyncio.create_subprocess_exec(*cmdline)
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


async def worker(the_q: asyncio.Queue, output_dir: str):
    try:
        while 1:
            my_task = the_q.get_nowait()
            await process_instance(my_task["instance_id"], output_dir)
    except asyncio.QueueEmpty:
        pass


async def main():
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=1)
    args = parser.parse_args()

    dataset = list(load_dataset('princeton-nlp/SWE-bench_Lite', split='test'))
    the_q = asyncio.Queue()
    for x in dataset:
        if "sphinx" in x["repo"]:
            await the_q.put(x)
    worker_tasks = [asyncio.create_task(worker(the_q, args.output_dir)) for _ in range(args.workers)]
    await asyncio.gather(*worker_tasks)
    assert the_q.empty()


if __name__ == "__main__":
    asyncio.run(main())
