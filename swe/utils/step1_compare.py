import re
import json
import whatthepatch

from pathlib import Path
from typing import Set, Dict


def patched_file(patch: str) -> str:
    files = list(whatthepatch.parse_patch(patch))
    assert len(files) == 1
    header = files[0].header
    assert header.old_path[len("a/"):] == header.new_path[len("b/"):]
    return header.old_path[len("a/"):]


def extract_filenames(text: str, filter_tests: bool = True) -> Set[str]:
    pattern = r'\b(?:[a-zA-Z]:\\|/)?(?:[\w-]+[/\\])*[\w-]+\.\w+\b'
    filenames = set(re.findall(pattern, text))
    if filter_tests:
        filenames = {f for f in filenames if "test" not in f.lower()}
    return filenames


def count_refact(filename: Path, pfilename: str, counters: Dict):
    prediction = json.loads(filename.read_text())
    summarized_problem_statement = prediction.get("summarized_problem_statement", "")
    if not summarized_problem_statement:
        return counters
    counters["summarized_instances"] += 1
    counters["num_filenames"].append(len(extract_filenames(summarized_problem_statement)))
    if Path(pfilename).name not in summarized_problem_statement:
        return counters
    counters["found_name_instances"] += 1
    if pfilename not in summarized_problem_statement:
        return counters
    counters["found_file_instances"] += 1
    return counters


def count_aider(filename: Path, pfilename: str, counters: Dict):
    prediction = json.loads(filename.read_text())
    model_patch = prediction.get("model_patch")
    if model_patch is None:
        return counters
    counters["summarized_instances"] += 1
    counters["num_filenames"].append(len(set(prediction["added_files"] + prediction["edited_files"])))
    if Path(pfilename).name not in model_patch:
        return counters
    counters["found_name_instances"] += 1
    if pfilename not in model_patch:
        return counters
    counters["found_file_instances"] += 1
    return counters


def count(filename: Path, pfilename: str, counters: Dict):
    counters = {
        "found_file_instances": 0,
        "found_name_instances": 0,
        "summarized_instances": 0,
        "num_filenames": [],
        **counters,
    }
    prediction = json.loads(filename.read_text())
    if "added_files" in prediction:
        return count_aider(filename, pfilename, counters)
    else:
        return count_refact(filename, pfilename, counters)


def print_counters(counters: Dict, total_instances: int):
    def _percent(v, t):
        return v / t * 100

    sinst = counters['summarized_instances']
    fninst = counters['found_name_instances']
    ffinst = counters['found_file_instances']

    print(f"Total instances:        {total_instances}")
    print(f"Summarized  instances:  {sinst} {_percent(sinst, total_instances):.2f}%")
    print(f"Found names:            {fninst} {_percent(fninst, total_instances):.2f}% / {_percent(fninst, sinst):.2f}%")
    print(f"Found files:            {ffinst} {_percent(ffinst, total_instances):.2f}% / {_percent(ffinst, sinst):.2f}%")
    print(f"Mean filenames in task: {sum(counters['num_filenames']) / len(counters['num_filenames']):.2f}")


if __name__ == "__main__":
    swe_bench_lite_filename = Path("swe/princeton-nlp--SWE-bench_Lite.json")
    # no_file_in_problem_instances = True
    no_file_in_problem_instances = False

    swe_predictions = {
        Path("swe/predictions/step1"): {},
        # Path("swe/predictions/step1-att3-gpt35"): {},
        Path("swe/predictions/step1-att3-gpt4"): {},
        Path("/home/mitya/projects/aider-swe-bench/predictions/multi-models--gpt-4o--openrouter-anthropic-claude-3-opus"): {},
    }
    total_instances = 0
    for instance in json.loads(swe_bench_lite_filename.read_text()):
        instance_id = instance["instance_id"]
        pfilename = patched_file(instance["patch"])

        def _filename(d: Path) -> Path:
            return d / f"{instance_id}.json"

        if any([not _filename(d).exists() for d in swe_predictions]):
            continue
        if no_file_in_problem_instances and Path(pfilename).name in instance["problem_statement"]:
            continue
        total_instances += 1
        for d, counters in swe_predictions.items():
            swe_predictions[d] = count(_filename(d), pfilename, counters)

    for d, counters in swe_predictions.items():
        print("=" * 40 + f" {d.name} " + "=" * 40)
        print_counters(counters, total_instances)
