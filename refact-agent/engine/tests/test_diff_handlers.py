import json
import os
import random
from copy import copy

import requests

from pathlib import Path

from termcolor import colored

current_dir = Path(__file__).parent.absolute()
test_file = current_dir / "test_file.py"


file_text = """
class Frog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
        elif self.x > pond_width:
            self.vx = -np.abs(self.vx)
        if self.y < 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
            self.vy = -np.abs(self.vy)

    def jump(self, pond_width, pond_height):
        self.x += self.vx * DT
        self.y += self.vy * DT
        self.bounce_off_banks(pond_width, pond_height)
        self.x = np.clip(self.x, 0, pond_width)
        self.y = np.clip(self.y, 0, pond_height)
"""[1:]


def diff_apply(payload):
    url = "http://localhost:8001/v1/diff-apply"
    response = requests.post(url, data=json.dumps(payload))
    assert response.status_code == 200
    return response.json()


def diff_preview(payload):
    url = "http://localhost:8001/v1/diff-preview"
    response = requests.post(url, data=json.dumps(payload))
    assert response.status_code == 200
    return response.json()


def diff_state(payload):
    url = "http://localhost:8001/v1/diff-state"
    response = requests.post(url, data=json.dumps(payload))
    assert response.status_code == 200
    return response.json()


payload1 = {
    "apply": [True, True, True, True],
    "chunks": [
        {
            "file_name": str(test_file),
            "file_action": "edit",
            "line1": 1,
            "line2": 4,
            "lines_remove": "\n".join(list(file_text.splitlines())[:6]),
            "lines_add": "# chunk0\n# chunk0\n"
        },
        {
            "file_name": str(test_file),
            "file_action": "edit",
            "line1": 8,
            "line2": 17,
            "lines_remove": "\n".join(list(file_text.splitlines())[7:16]),
            "lines_add": "# chunk1\n# chunk1\n"
        },
        {
            "file_name": str(test_file),
            "file_action": "edit",
            "line1": 18,
            "line2": 20,
            "lines_remove": "\n".join(list(file_text.splitlines())[17:23]),
            "lines_add": "# chunk2\n# chunk2"
        },
        {
            "file_name": str(test_file),
            "file_action": "edit",
            "line1": 18,
            "line2": 20,
            "lines_remove": "some random text",
            "lines_add": "# chunk3\n# chunk3\n"
        },
    ]
}


def test1():
    # applying all chunks all-together and then un-applying them all by once

    must_look_like = "# chunk0\n# chunk0\n\n# chunk1\n# chunk1\n\n# chunk2\n# chunk2\n"
    payload = copy(payload1)

    with test_file.open("w") as f:
        f.write(file_text)

    resp = diff_apply(payload)

    assert [r['applied'] for r in resp] == [True, True, True, False]
    assert [r['success'] for r in resp] == [True, True, True, False]

    assert test_file.read_text() == must_look_like

    payload["apply"] = [False] * len(payload["chunks"])
    resp = diff_apply(payload)

    assert [r['applied'] for r in resp] == [False, False, False, False]
    assert [r['success'] for r in resp] == [True, True, True, True]

    assert test_file.read_text() == file_text

    print(colored("test1 PASSED", "green"))


def test2():
    # applying and un-applying chunks one by one

    payload = copy(payload1)

    for i in range(len(payload["chunks"])):
        vec = [i == j for j in range(len(payload["chunks"]))]
        payload["apply"] = vec

        with test_file.open("w") as f:
            f.write(file_text)

        resp = diff_apply(payload)
        if i != 3:
            assert [r['applied'] for r in resp] == [v == 1 for v in vec]
        else:
            assert [r['applied'] for r in resp] == [False, False, False, False]
            assert [r['success'] for r in resp] == [True, True, True, False]

        payload["apply"] = [False] * len(payload["chunks"])
        resp = diff_apply(payload)

        assert [r['applied'] for r in resp] == [False, False, False, False]
        assert test_file.read_text() == file_text

    print(colored("test2 PASSED", "green"))


def test4():
    payload = copy(payload1)
    del payload["apply"]

    state = diff_state(payload)
    assert state["can_apply"] == [True, True, True, False]

    print(colored("test4 PASSED", "green"))


file_text1 = """
class Frog:
    def __init__(self, x, y, vx, vy):
        self.vx = vx
"""

file_text1_must_be = """
class Frog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy
"""

payload2 = {
    "apply": [True, True],
    "chunks": [
        {
            "file_name": str(test_file),
            "file_action": "edit",
            "line1": 4,
            "line2": 4,
            "lines_remove": "",
            "lines_add": "        self.x = x\n        self.y = y\n"
        },
        {
            "file_name": str(test_file),
            "file_action": "edit",
            "line1": 5,
            "line2": 5,
            "lines_remove": "",
            "lines_add": "        self.vy = vy\n"
        }
    ]
}


def test5():
    payload = copy(payload2)

    with test_file.open("w") as f:
        f.write(file_text1)

    _resp = diff_apply(payload)

    assert test_file.read_text() == file_text1_must_be
    print(colored("test5 PASSED", "green"))


payload_test_other = {
    "apply": [True, True, True],
    "chunks": [
        # TP
        {
            "file_name": str(test_file) + ".txt",
            "file_action": "add",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": "TEST"
        },
        {
            "file_name": str(test_file),
            "file_action": "remove",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": "TEST"
        },
        {
            "file_name": str(test_file) + '.txt',
            "file_action": "rename",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": "TEST",
            "file_name_rename": str(test_file)
        },
    ]
}

def safe_remove(file_path):
    try:
        if os.path.isfile(file_path):
            os.remove(file_path)
        elif os.path.isdir(file_path):
            os.rmdir(file_path)
    except FileNotFoundError:
        pass
    except OSError as e:
        print(f"Error: {e.strerror}")


def safe_create(file_path):
    try:
        os.makedirs(file_path)
    except FileExistsError:
        pass


def test6():
    payload = copy(payload_test_other)
    safe_remove(str(test_file) + ".abc")
    safe_remove(str(test_file) + ".txt")

    del payload["apply"]

    with test_file.open("w") as f:
        f.write(file_text1)

    resp = diff_state(payload)

    assert resp['can_apply'] == [True, True, True], resp

    print(colored("test6 PASSED", "green"))


payload_test_other1 = {
    "apply": [True, True, True, True, True],
    "chunks": [
        {
            "file_name": str(test_file) + ".1.test",
            "file_action": "add",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": "TEST"
        },
        {
            "file_name": str(test_file.parent / "dir1" / ".1.test"),
            "file_action": "add",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": "TEST"
        },
        {
            "file_name": str(test_file.parent / "dir1" / "dir2" / ".1.test"),
            "file_action": "add",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": "TEST"
        },
        {
            "file_name": str(test_file) + ".2.test",
            "file_action": "remove",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": "TEST"
        },
        {
            "file_name": str(test_file) + '.3.test_rename',
            "file_action": "rename",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": "TEST",
            "file_name_rename": str(test_file) + ".3.test"
        },
    ]
}


def test7():
    def init():
        safe_remove(str(test_file) + ".1.test")
        safe_remove(str(test_file.parent / "dir1" / "dir2" / ".1.test"))
        safe_remove(str(test_file.parent / "dir1" / "dir2"))
        safe_remove(str(test_file.parent / "dir1" / ".1.test"))
        safe_remove(str(test_file.parent / "dir1"))
        safe_remove(str(test_file) + ".2.test")
        safe_remove(str(test_file) + ".3.test")
        safe_remove(str(test_file) + '.3.test_rename')

    init()
    # create files
    with open(str(test_file) + ".2.test", "w") as f:
        f.write("TEST")
    with open(str(test_file) + ".3.test", "w") as f:
        f.write("TEST")

    payload = copy(payload_test_other1)

    res = diff_apply(payload)

    assert (test_file.parent / "dir1" / "dir2" / ".1.test").name in os.listdir(test_file.parent / "dir1" / "dir2")
    assert (test_file.parent / "dir1" / ".1.test").name in os.listdir(test_file.parent / "dir1")

    assert [r['applied'] for r in res] == [True, True, True, True, True], res
    assert [r['success'] for r in res] == [True, True, True, True, True], res

    init()

    print(colored("test7 PASSED", "green"))


payload_test_dirs_TP = {
    "apply": [True, True, True, True],
    "chunks": [
        {
            "file_name": str(current_dir / "test_dir1"),
            "file_action": "add",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": ""
        },
        {
            "file_name": str(current_dir / "test_dir11" / "test_dir12"),
            "file_action": "add",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": ""
        },
        {
            "file_name": str(current_dir / "test_dir2"),
            "file_action": "remove",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": ""
        },
        {
            "file_name": str(current_dir / "test_dir3"),
            "file_action": "rename",
            "line1": 1,
            "line2": 1,
            "lines_remove": "",
            "lines_add": "",
            "file_name_rename": str(current_dir / "test_dir4")
        },
    ]
}


def test8():
    def init():
        safe_remove(current_dir / "test_dir1")
        safe_create(current_dir / "test_dir2")
        safe_remove(current_dir / "test_dir3")
        safe_remove(current_dir / "test_dir11" / "test_dir12")
        safe_remove(current_dir / "test_dir11")
        safe_create(current_dir / "test_dir4")

    init()

    payload = copy(payload_test_dirs_TP)
    del payload["apply"]

    resp = diff_state(payload)

    assert resp['can_apply'] == [True, True, True, True], resp

    payload1 = copy(payload_test_dirs_TP)

    resp = diff_apply(payload1)
    assert [r['applied'] for r in resp] == [True, True, True, True], resp

    assert (current_dir / "test_dir1").name in os.listdir(current_dir), os.listdir(current_dir)
    assert (current_dir / "test_dir11" / "test_dir12").name in os.listdir((current_dir / "test_dir11"))
    assert (current_dir / "test_dir2").name not in os.listdir(current_dir)
    assert (current_dir / "test_dir3").name in os.listdir(current_dir)
    assert (current_dir / "test_dir4").name not in os.listdir(current_dir)

    print(colored("test8 PASSED", "green"))


FILE0 = """import numpy as np

DT = 0.01

class Frog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
        elif self.x > pond_width:
            self.vx = -np.abs(self.vx)
        if self.y < 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
            self.vy = -np.abs(self.vy)

    def jump(self, pond_width, pond_height):
        self.x += self.vx * DT
        self.y += self.vy * DT
        self.bounce_off_banks(pond_width, pond_height)
        self.x = np.clip(self.x, 0, pond_width)
        self.y = np.clip(self.y, 0, pond_height)

"""

FILE1 = """import numpy as np

DT = 0.01



class AnotherFrog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
        elif self.x > pond_width:
            self.vx = -np.abs(self.vx)
        if self.y < 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
            self.vy = -np.abs(self.vy)

    def jump(self, pond_width, pond_height):
        self.x += self.vx * DT
        self.y += self.vy * DT
        self.bounce_off_banks(pond_width, pond_height)
        self.x = np.clip(self.x, 0, pond_width)
        self.y = np.clip(self.y, 0, pond_height)

"""

frog_payload = {
    "apply": [True, True],
    "chunks": [
        {
            "file_name": "/tmp/frog.py",
            "file_action": "edit",
            "line1": 5,
            "line2": 6,
            "lines_remove": "class Frog:\n",
            "lines_add": "class AnotherFrog:\n"
        },
        {
            "file_name": "/tmp/frog.py",
            "file_action": "edit",
            "line1": 4,
            "line2": 4,
            "lines_remove": "",
            "lines_add": "\n\n"
        }
    ]
}


def test9():
    with open("/tmp/frog.py", "w") as f:
        f.write(FILE0)

    resp = diff_preview(frog_payload)

    assert [r["applied"] for r in resp["state"]] == [True, True], resp
    assert len(resp["results"]) == 1, resp
    r0 = resp["results"][0]

    assert r0["file_name_edit"] == "/tmp/frog.py"
    assert r0["file_text"] == FILE1

    print(colored("test9 PASSED", "green"))


def main():
    test1()
    test2()
    test4()
    test5()
    test6()
    test7()
    test8()
    test9()


if __name__ == "__main__":
    main()
