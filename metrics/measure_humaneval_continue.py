import sys, termcolor, subprocess, json, time, random
from copy import deepcopy
from mpi4py import MPI
from human_eval.data import write_jsonl, read_problems
from human_eval.data import read_problems
import requests


#MODEL = "smallcloudai/Refact-1_6B-fim"
MODEL = "Refact/1.6B"

TEMPERATURE = 0.2
TOP_P = 0.95
TIMES = 1
MAX_TOKENS = 256


def run_completion_call(src_txt):
    res = requests.post(f"http://127.0.0.1:8008/v1/completions", json={
        "model": MODEL,
        "max_tokens": MAX_TOKENS,
        "stream": False,
        "echo": True,
        "top_p": TOP_P,
        "temperature": TEMPERATURE,
        "prompt": src_txt,
        "stop": ["\n\n\n"],
    })
    res.raise_for_status()
    j = res.json()
    # print(j)
    return j["choices"][0]["text"]


def test_by_continuing(comm, case):
    orig = case["prompt"].rstrip()
    print_me = termcolor.colored(orig[:-1], "yellow")
    if comm.size == 1:
        print(print_me)
    t = run_completion_call(orig)
    uncut = t
    lines = t.split("\n")
    filtered = []
    for x in lines:
        if x.startswith(" ") or x.strip() == "":
            filtered.append(x)
        elif not x.startswith(" "):
            break
    t = "\n".join(filtered)
    assert uncut.startswith(t)
    print_response = termcolor.colored(t, "green") + " " + termcolor.colored(uncut[len(t):], attrs=["dark"])
    if comm.size == 1:
        print(print_response)
    else:
        print(print_me + "\n" + print_response)
    case["completion"] = t


if __name__ == "__main__":
    postfix = ""
    if len(sys.argv) > 1:
        postfix = sys.argv[1]
    t0 = time.time()
    from human_eval.data import write_jsonl, read_problems
    from human_eval.data import read_problems
    problems = list(read_problems().values()) * TIMES
    comm = MPI.COMM_WORLD
    my_problems = problems[comm.rank::comm.size]
    output = []
    for i, case_ in enumerate(my_problems):
        case = deepcopy(case_)
        print("-" * 40, " rank=%i case=%i" % (comm.rank, i), "-" * 40)
        test_by_continuing(comm, case)
        output.append(case)
    comm.barrier()
    t1 = time.time()
    tmp = comm.gather(output, root=0)
    if comm.rank == 0:
        all_output = [x for y in tmp for x in y]
        output_name = "human-%s%s.jsonl" % ("continue", postfix)
        write_jsonl(output_name, all_output)
        res = subprocess.check_output(f"evaluate_functional_correctness {output_name}", shell=True)
        metrics = json.loads(res.decode('utf-8').strip().split('\n')[-1].replace("'", '"'))
        print(termcolor.colored(metrics, "magenta"))
        tmp = "method=%s temperature=%0.2f top_p=%0.2f postfix='%s' world=%i times=%i  %s %0.2fs %s\n" % (
            "continue", TEMPERATURE, TOP_P, postfix, comm.size, TIMES, metrics, (t1 - t0), MODEL)
        with open("human-eval-all-results.txt", "a") as f:
            f.write(tmp)
        print(tmp)
