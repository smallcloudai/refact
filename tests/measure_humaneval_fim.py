import sys, termcolor, subprocess, json, time, random
from copy import deepcopy
from mpi4py import MPI
from human_eval.data import write_jsonl, read_problems
from human_eval.data import read_problems
import requests


MODEL = "smallcloudai/Refact-1_6B-fim"
MODEL = "Refact/1.6B"

TEMPERATURE = 0.2
TOP_P = 0.95
TIMES = 1
MAX_TOKENS = 256


def make_call(src_py, src_txt, cursor_line, cursor_pos):
    res = requests.post(f"http://127.0.0.1:8001/v1/code-completion", json={
        "inputs": {
            "sources": {src_py: src_txt},
            "cursor": {"file": src_py, "line": cursor_line, "character": cursor_pos},
            "multiline": True
        },
        "stream": False,
        "model": MODEL,
        "parameters": {
            "temperature": TEMPERATURE,
            "max_new_tokens": MAX_TOKENS
        }
    })
    res.raise_for_status()
    j = res.json()
    print(j)
    return j["choices"][0]["code_completion"]


def test_by_infill(case):
    orig = case["prompt"]
    while orig[-1] == "\n":
        orig = orig[:-1]
    last_line = "   "
    cursor_line = orig.count("\n") + 1
    cursor_pos = len(last_line)
    send = orig + "\n" + last_line
    code_completion = make_call("test.py", send, cursor_line=cursor_line, cursor_pos=cursor_pos)
    print(termcolor.colored(send, "yellow") + termcolor.colored(code_completion, "green"))
    dest = send + code_completion
    assert dest.startswith(orig)
    case["completion"] = dest[len(orig):]


if __name__ == "__main__":
    postfix = ""
    if len(sys.argv) > 1:
        postfix = sys.argv[1]
    t0 = time.time()
    problems = list(read_problems().values()) * TIMES
    comm = MPI.COMM_WORLD
    my_problems = problems[comm.rank::comm.size]
    output = []
    for i, case_ in enumerate(my_problems):
        case = deepcopy(case_)
        print("-" * 40, " rank=%i case=%i" % (comm.rank, i), "-" * 40)
        test_by_infill(case)
        output.append(case)
    comm.barrier()
    t1 = time.time()
    print("rank=%i len(output)==%i" % (comm.rank, len(output)))
    tmp = comm.gather(output, root=0)
    if comm.rank == 0:
        all_output = [x for y in tmp for x in y]
        print("len(all_output)==%i" % (len(all_output),))
        output_name = "human-%s%s.jsonl" % ("fim", postfix)
        write_jsonl(output_name, all_output)
        res = subprocess.check_output(f"evaluate_functional_correctness {output_name}", shell=True)
        metrics = json.loads(res.decode('utf-8').strip().split('\n')[-1].replace("'", '"'))
        print(termcolor.colored(metrics, "magenta"))
        tmp = "method=%s temperature=%0.2f top_p=%0.2f postfix='%s' world=%i times=%i  %s %0.2fs %s\n" % (
            "fim", TEMPERATURE, TOP_P, postfix, comm.size, TIMES, metrics, (t1 - t0), MODEL)
        with open("human-eval-all-results.txt", "a") as f:
            f.write(tmp)
        print(tmp)
