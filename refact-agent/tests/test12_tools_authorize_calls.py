import requests
import termcolor


def test(body: dict, expected: dict):
    response = requests.post(
        "http://127.0.0.1:8001/v1/tools-check-if-confirmation-needed",
        json=body,
        headers={
            "Content-Type": "application/json",
        },
    )
    assert (
        response.status_code == expected["status_code"]
    ), f"Status code is {response.status_code}, expected {expected['status_code']}"
    if response.status_code == 200:
        assert (
            response.json().get("pause") == expected["pause"]
        ), f"Pause is {response.json().get('pause')}, expected {expected['pause']}"
        print(termcolor.colored(response.json().get("pause_reasons"), "cyan"))
        assert (
            len(response.json().get("pause_reasons")) == expected["pause_reasons_len"]
        ), f"Pause reasons len is {len(response.json().get('pause_reasons'))}, expected {expected['pause_reasons_len']}"
    print(termcolor.colored("PASS", "green"))


def test_paused():
    body_paused = {
        "tool_calls": [
            {
                "id": "1",
                "function": {
                    "name": "github",
                    "arguments": '{"command": "repo delete", "project_dir": "/home/user/my_dir"}',
                },
                "type": "github",
            },
            {
                "id": "2",
                "function": {
                    "name": "github",
                    "arguments": '{"command": "auth token", "project_dir": "/home/user/my_dir"}',
                },
                "type": "github",
            },
            {
                "id": "3",
                "function": {
                    "name": "github",
                    "arguments": '{"command": "repo list", "project_dir": "/home/user/my_dir"}',
                },
                "type": "github",
            },
        ],
    }
    expected = {
        "status_code": 200,
        "pause": True,
        "pause_reasons_len": 2,
    }
    test(body_paused, expected)


def test_ok():
    body_ok = {
        "tool_calls": [
            {
                "id": "3",
                "function": {
                    "name": "github",
                    "arguments": '{"command": "repo list", "project_dir": "/home/user/my_dir"}',
                },
                "type": "github",
            },
        ],
    }
    expected = {
        "status_code": 200,
        "pause": False,
        "pause_reasons_len": 0,
    }
    test(body_ok, expected)


def test_error():
    body_error = {
        "tool_calls": [
            {
                "id": "3",
                "function": {
                    "name": "github",
                    "arguments": '{"project_dir": "/home/user/my_dir"}',
                },
                "type": "github",
            },
        ],
    }
    expected = {
        "status_code": 422,
    }
    test(body_error, expected)


if __name__ == "__main__":
    test_paused()
    test_ok()
    test_error()
