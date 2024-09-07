import json

from pathlib import Path
from collections import Counter

from typing import Dict, Set


def collect_competitor(root: Path, instances: Set[str]):
    total_instances = 0
    unresolved_instances: Set = set()
    resolved_instances: Set = set()
    for filename in (root / "logs").glob("*.log"):
        instance_id = filename.name.split(".")[0]
        if instance_id not in instances:
            print(f"impostor in {root}: {instance_id}")
            continue
        total_instances += 1
        if "All Tests Passed" in filename.read_text():
            resolved_instances.add(instance_id)
        else:
            unresolved_instances.add(instance_id)
    return total_instances, resolved_instances, unresolved_instances


def main():
    swe_bench_lite_tasks = Path(__file__).parent / "princeton-nlp--SWE-bench_Lite.json"
    swe_bench_lite_instances = set(
        instance["instance_id"]
        for instance in json.loads(swe_bench_lite_tasks.read_text())
    )
    lite_board_logs = Path('/home/mitya/projects/swe-experiments/evaluation/lite')
    all_instances: Set = set()
    easy_instances_counter: Counter = Counter()
    hard_instances_counter: Counter = Counter()
    competitors_info: Dict[str, Dict[str, int]] = {}
    for d in lite_board_logs.iterdir():
        if not d.is_dir():
            continue
        # date = d.name.split("_")[0]
        competitor = "_".join(d.name.split("_")[1:])
        total_instances, resolved_instances, unresolved_instances = collect_competitor(d, swe_bench_lite_instances)
        all_instances.update([*resolved_instances, *unresolved_instances])
        easy_instances_counter.update(resolved_instances)
        hard_instances_counter.update(unresolved_instances)
        competitors_info[competitor] = {
            "total_instances": total_instances,
            "resolved_instances": len(resolved_instances),
            # "resolved_instances_percent": len(resolved_instances) / total_instances * 100,
            "resolved_instances_percent": len(resolved_instances) / len(swe_bench_lite_instances) * 100,
        }
    print()
    print("Competitors")
    for name, info in sorted(competitors_info.items(), key=lambda x: x[1]["resolved_instances_percent"]):
        full_set_message = "full"
        if info["total_instances"] != len(swe_bench_lite_instances):
            full_set_message = f"{info['total_instances'] / len(swe_bench_lite_instances) * 100.:.3f}%"
        print(f"{name:<40} {full_set_message:<10} {info['resolved_instances_percent']:.3f}%")
    print()
    print("Hard instances")
    for name, unresolved in hard_instances_counter.most_common(50):
        print(f"{name:<40} {unresolved / len(competitors_info) * 100:.3f} {len(competitors_info) - unresolved}")
    assert len(all_instances) == len(swe_bench_lite_instances)


if __name__ == '__main__':
    main()
