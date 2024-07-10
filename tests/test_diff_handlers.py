import json
import random
from copy import copy

import requests

from pathlib import Path

from termcolor import colored

current_dir = Path(__file__).parent.absolute()
test_file = current_dir / "test_file.rs.temp"


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
    return json.loads(response.text)


def diff_state(payload):
    url = "http://localhost:8001/v1/diff-state"
    response = requests.post(url, data=json.dumps(payload))
    assert response.status_code == 200
    return json.loads(response.text)


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

    assert resp["state"] == [1, 1, 1, 2]
    fuzzy_results = resp["fuzzy_results"]
    fuzzy_results.sort(key=lambda x: x['chunk_id'])
    assert [f["fuzzy_n_used"] for f in fuzzy_results] == [3, 0, 4]

    assert test_file.read_text() == must_look_like

    payload["apply"] = [False] * len(payload["chunks"])
    resp = diff_apply(payload)

    assert resp['state'] == [0, 0, 0, 0]
    assert resp['fuzzy_results'] == []

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
            assert resp["state"] == vec
        else:
            assert resp["state"] == [0, 0, 0, 2]

        payload["apply"] = [False] * len(payload["chunks"])
        resp = diff_apply(payload)
        assert resp['state'] == [0, 0, 0, 0]
        assert test_file.read_text() == file_text

    print(colored("test2 PASSED", "green"))


def test3():
    # applying and un-applying a random amount of chunks 100 times

    payload = copy(payload1)
    with test_file.open("w") as f:
        f.write(file_text)

    for iter_idx in range(100):
        chunks_n_to_apply = random.randint(1, len(payload["chunks"]))
        chunks_ids_to_apply = random.sample(list(range(len(payload['chunks']))), chunks_n_to_apply)
        chunks_ids_to_apply.sort()

        vec = [i in chunks_ids_to_apply for i in range(len(payload["chunks"]))]
        payload["apply"] = vec
        err_msg = f"iter_idx={iter_idx}, chunks_ids_to_apply={chunks_ids_to_apply}, vec={vec}"

        resp = diff_apply(payload)
        if 3 not in chunks_ids_to_apply:
            assert resp["state"] == vec, err_msg
        else:
            assert resp["state"] == [*vec[:-1], 2], err_msg

        payload["apply"] = [False] * len(payload["chunks"])
        resp = diff_apply(payload)
        assert resp['state'] == [0, 0, 0, 0]

        assert test_file.read_text() == file_text

    print(colored("test3 PASSED", "green"))


def test4():
    payload = copy(payload1)
    del payload["apply"]

    state = diff_state(payload)
    print(state)
    assert state["can_apply"] == [True, True, True, False]

    print(colored("test4 PASSED", "green"))


def main():
    test1()
    # test2()
    # test3()
    # test4()


if __name__ == "__main__":
    main()
