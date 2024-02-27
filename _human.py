from human_eval.data import read_problems
problems = list(read_problems().values())
with open("_problem0.prompt.py", "w") as f:
    f.write(problems[0]["prompt"])
with open("_problem0.test.py", "w") as f:
    f.write(problems[0]["test"])
