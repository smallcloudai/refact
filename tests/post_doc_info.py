import json
import time
import hashlib
import requests
from typing import Dict, Optional, List

TEST = "my.json"
COMPARE_WITH_TEST = "their.json"
BASE_URL = "http://localhost:8001/v1"


def compare_with_data() -> Optional[Dict]:
    try:
        with open(COMPARE_WITH_TEST, "r") as f:
            data = json.loads(f.read())
            return data if isinstance(data, dict) else None
    except Exception:
        return None


def at_command_preview(query: str, model: str = "llama2/7b") -> Dict:
    payload = {
        "query": query,
        "model": model,
    }
    time_start = time.time()
    resp = requests.post(
        f"{BASE_URL}/at-command-preview",
        data=json.dumps(payload),
    )
    if resp.status_code != 200:
        raise Exception(f"at-command-preview failed with status code {resp.status_code}: {resp.text}")
    data = json.loads(resp.text)
    done_time = time.time() - time_start
    return {
        "data": data,
        "done_time": round(done_time, 3),
        "hash": hashlib.md5(resp.text.encode()).hexdigest()
    }


def check(n_tests: int, queries: List[str], compare_str: str, compare_with: Optional[Dict] = None, detailed: bool = False):
    def compare():
        print(f"COMPARE: {compare_str}")
        ex_res = {}
        ex_compare = {}

        for t in results:
            for k, v in t.items():
                if k.startswith("case"):
                    ex_res.setdefault(k, []).append({
                        "done_time": v["done_time"],
                        "hash": v["hash"]
                    })

        for t in (compare_with or {}).get(compare_str, []):
            for k, v in t.items():
                if k.startswith("case"):
                    ex_compare.setdefault(k, []).append({
                        "done_time": v["done_time"],
                        "hash": v["hash"]
                    })

        for k, v in ex_res.items():
            if v:
                hash_consistent = {i['hash'] for i in v}.__len__() == 1
                print(f"TestCase: {k}; n_tests: {len(v)}; average_time: {sum([i['done_time'] for i in v])/len(v):.3f}; hash_consistent: {hash_consistent}")
            else:
                print(f"TestCase {k}: Empty")

        for k, v in ex_compare.items():
            if v:
                hash_consistent = {i['hash'] for i in v}.__len__() == 1
                same_hash = False
                if ex_res.get(k):
                    if set(ex_res[k][0]['hash']) == set(v[0]['hash']):
                        same_hash = True
                print(f"CompareCase: {k}; n_tests: {len(v)}; average_time: {sum([i['done_time'] for i in v])/len(v):.3f}; hash_consistent: {hash_consistent}; same_hash: {same_hash}")
            else:
                print(f"CompareCase {k}: Empty")

    results = []

    for i in range(n_tests):
        result = {"n": i}
        for j, query in enumerate(queries):
            result[f"case_{j+1}"] = at_command_preview(query)
            if detailed:
                print(json.dumps(result[f"case_{j+1}"], indent=4))
        results.append(result)

    compare()
    return results


def main():
    n_tests = 3
    res = {
        "at_file": check(n_tests, ["@file forward_to_hf_endpoint.rs", "@file forward_to_hf_endpoint.rs:10-13"], "at_file", compare_with_data()),
        "at_workspace": check(n_tests, ["@workspace abc", "@workspace bcd"], "at_workspace", compare_with_data()),
        "at_references": check(n_tests, ["@references bearer", "@references serde_json"], "at_references", compare_with_data()),
        "at_definition": check(n_tests, ["@definition forward_to_hf_style_endpoint", "@definition get_embedding_hf_style"], "at_definition", compare_with_data()),
    }

    with open(TEST, "w") as f:
        json.dump(res, f, indent=4)
    # check(1, ["@workspace abc", "@workspace bcd"], "at_workspace", compare_with_data(), True)


if __name__ == "__main__":
    main()
