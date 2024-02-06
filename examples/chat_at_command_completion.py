import json
import requests
import termcolor


url = "http://localhost:8001/v1/at-command-completion"


def test_at_command_completion(query):
    query_real = query.replace("<C>", "")
    payload = json.dumps({
        "query": query_real,
        "cursor": query.find("<C>"),
        "top_n": 3,
    })
    response = requests.post(url, data=payload)
    print(payload)
    print(termcolor.colored(response.text, 'red'))
    j = json.loads(response.text)
    r = j["replace"]
    if len(j["completions"]) > 0:
        query_completed = query_real[:r[0]] + j["completions"][0] + query_real[r[1]:]
        print(query_completed)
    else:
        print("no completions")


test_at_command_completion("""
other line -3
other line -2
other line -1
@file deltadelta.rs<C>
other line 1
other line 2
other line 3
""")

test_at_command_completion("""
other line -3
other line -2
other line -1
@work<C>
other line 1
other line 2
other line 3
""")


