import json
import requests

from typing import Dict


def at_completion_post(query: str) -> Dict:
    payload = {
        "query": query.replace("|", ""),
        "cursor": query.find("|"),
        "top_n": 50,
    }
    response = requests.post(
        "http://localhost:8001/v1/at-command-completion",
        data=json.dumps(payload),
    )
    assert response.status_code == 200, f"Response code: {response.status_code}: {response.text}"

    decoded = json.loads(response.text)

    return decoded


def test1():
    query = """
@|
    """
    resp = at_completion_post(query)
    assert resp["replace"] == [1, 2]
    assert all(x.endswith(" ") for x in resp["completions"])


def test2():
    query = """
some text over here @|
    """
    resp = at_completion_post(query)
    assert resp["replace"] == [21, 22]


def test3():
    query = """
@file abc and @fi|
    """
    resp = at_completion_post(query)
    assert resp["replace"] == [15, 18]
    assert resp["completions"] == ["@file "]


def test4():
    query = """
@file abc and @file |
    """
    resp = at_completion_post(query)
    assert resp["replace"] == [21, 21]
    assert len(resp["completions"]) != 0


def main():
    tests = [test1, test2, test3, test4]
    for test in tests:
        test()
    print("PASS")


if __name__ == "__main__":
    main()
