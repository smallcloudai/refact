import re
import json
import whatthepatch

from pathlib import Path
from typing import Set


def patched_file(patch: str) -> str:
    files = list(whatthepatch.parse_patch(patch))
    assert len(files) == 1
    header = files[0].header
    assert header.old_path[len("a/"):] == header.new_path[len("b/"):]
    return header.old_path[len("a/"):]


def extract_filenames(text: str) -> Set[str]:
    py_pattern = r"(?:[^/ ]+/)*[^/ ]+\.py"
    return set(re.findall(py_pattern, text))


if __name__ == "__main__":
    swe_bench_lite_filename = Path("swe/princeton-nlp--SWE-bench_Lite.json")
    # swe_predictions_dir = Path("swe/predictions/gpt35-gpt4")
    swe_predictions_dir = Path("swe/predictions/step1")
    # no_file_in_problem_instances = True
    no_file_in_problem_instances = False

    found_file_instances = 0
    found_name_instances = 0
    summarized_instances = 0
    total_instances = 0
    num_filenames = []
    for instance in json.loads(swe_bench_lite_filename.read_text()):
        instance_id = instance["instance_id"]
        filename = patched_file(instance["patch"])
        prediction_filename = swe_predictions_dir / f"{instance_id}.json"
        if not prediction_filename.exists():
            continue
        if no_file_in_problem_instances and Path(filename).name in instance["problem_statement"]:
            continue
        total_instances += 1
        prediction = json.loads(prediction_filename.read_text())
        summarized_problem_statement = prediction.get("summarized_problem_statement")
        if summarized_problem_statement is None:
            continue
        summarized_instances += 1
        num_filenames.append(len(extract_filenames(summarized_problem_statement)))
        if Path(filename).name not in summarized_problem_statement:
            continue
        found_name_instances += 1
        if filename not in summarized_problem_statement:
            continue
        found_file_instances += 1

    print(f"Total instances:        {total_instances}")
    print(f"Summarized  instances:  {summarized_instances} {summarized_instances / total_instances * 100:.2f}%")
    print(f"Found names:            {found_name_instances} {found_name_instances / total_instances * 100:.2f}% / {found_name_instances / summarized_instances * 100:.2f}%")
    print(f"Found files:            {found_file_instances} {found_file_instances / total_instances * 100:.2f} / {found_file_instances / summarized_instances * 100:.2f}%")
    print(f"Mean filenames in task: {sum(num_filenames) / len(num_filenames):.2f}")
