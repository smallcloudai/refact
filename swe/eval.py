import asyncio
import jsonlines

from pathlib import Path


def all_tests_passed(instance_id: str, log_dir: Path):
    instance_logs = list(log_dir.glob(f"{instance_id}*.log"))
    assert len(instance_logs) < 2
    if not instance_logs:
        return False
    return "All Tests Passed" in instance_logs[0].read_text()


async def main():
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--workers", type=int, default=8)
    parser.add_argument("--run", type=str, default="gpt35-gpt4")
    args = parser.parse_args()

    swe_bench_eval = "/home/mitya/projects/aider-swe-bench/SWE-bench-docker/run_evaluation.py"
    swe_bench_tasks = Path(__file__).parent / "princeton-nlp--SWE-bench_Lite.json"
    log_dir = Path(__file__).parent / "logs" / args.run
    predictions_path = Path(__file__).parent / "predictions" / args.run / "all_preds.jsonl"

    assert predictions_path.exists()
    log_dir.mkdir(exist_ok=True, parents=True)
    try:
        process = await asyncio.create_subprocess_exec(
            "python", swe_bench_eval,
            "--skip_existing", "--num_processes", "8",
            "--swe_bench_tasks", str(swe_bench_tasks),
            "--log_dir", str(log_dir),
            "--predictions_path", str(predictions_path),
        )
        await process.communicate()
    except Exception as e:
        print(f"failed to eval {args.run}: {e or type(e)}")
        exit(1)

    total_instances = 300
    instance_processed = 0
    other_error = 0
    step1_error = 0
    step2_error = 0
    patch_produced = 0
    problem_solved = 0
    with jsonlines.open(predictions_path, "r") as reader:
        for instance in reader:
            instance_processed += 1

            if instance.get("error") is not None:
                if "step1" in instance.get("error"):
                    step1_error += 1
                elif "step2" in instance.get("error"):
                    step2_error += 1
                else:
                    other_error += 1
                continue

            has_patch = bool(instance["model_patch"])
            if not has_patch:
                continue
            patch_produced += 1

            solved = all_tests_passed(instance["instance_id"], log_dir)
            if not solved:
                continue
            problem_solved += 1
    total_errors = step1_error + step2_error + other_error
    no_error_no_patch = total_instances - total_errors - patch_produced

    print(f"processed {instance_processed}/{total_instances} instances")
    print(f"step1 error: {step1_error} ({step1_error / instance_processed * 100:.2f}%) problems")
    print(f"step2 error: {step2_error} ({step2_error / instance_processed * 100:.2f}%) problems")
    print(f"other error: {other_error} ({other_error / instance_processed * 100:.2f}%) problems")
    print(f"total errors: {total_errors} ({total_errors / instance_processed * 100:.2f}%) problems")
    print(f"no error no patch: {no_error_no_patch} ({no_error_no_patch / total_instances * 100:.2f}%) problems")
    print(f"produced {patch_produced} ({patch_produced / instance_processed * 100:.2f}%) patches")
    print(f"solved {problem_solved} ({problem_solved / instance_processed * 100:.2f}%) problems")
    print(f"solved {problem_solved / patch_produced * 100:.2f}% of patched problems")


if __name__ == "__main__":
    asyncio.run(main())
