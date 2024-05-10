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


def main():
    tests = [test1, test2]
    for test in tests:
        test()
    print("All tests passed!")


if __name__ == "__main__":
    main()