import requests
import json
import pathlib
from termcolor import colored

# TODO: SecretaryBird


BASE_DIR = pathlib.Path(__file__).parent.resolve()
FROG_PY = BASE_DIR / "emergency_frog_situation" / "frog.py"
TEST11_DATA = BASE_DIR / "test11_data"
TOAD_ORIG = BASE_DIR / "test11_data" / "toad_orig.py"


def patch_request(messages, ticket_ids):
    payload = {
        "messages": messages,
        "ticket_ids": ticket_ids,
    }
    resp = requests.post(
        "http://localhost:8001/v1/patch-single-file-from-ticket",
        data=json.dumps(payload),
    )
    assert resp.status_code == 200, resp.text
    return resp.json()


def make_messages(ticket_text: str):
    return [
        {"role": "assistant", "content": ticket_text}
    ]


def test01_rewrite_whole_file():
    text_expected = "# FROG"
    ticket_text = \
f"""📍REPLACE_FILE 001 {FROG_PY}
```python
{text_expected}
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])

    res0 = resp["results"][0]
    assert res0["file_name_edit"] == str(FROG_PY)
    assert res0["file_text"] == text_expected, res0["file_text"]
    print(colored("test01_rewrite_whole_file PASSED", "green"))


def test01_new_file():
    text_expected = "# FROG"
    FN = str(FROG_PY) + ".temp"
    ticket_text = \
f"""📍NEW_FILE 001 {FN}
```python
{text_expected}
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])

    res0 = resp["results"][0]
    assert res0["file_name_add"] == str(FN)
    assert res0["file_text"] == text_expected, res0["file_text"]
    print(colored("test01_new_file PASSED", "green"))


def test01_partial_edit():
    text_expected = (TEST11_DATA / "toad_partial_edit_01.py").read_text()
    ticket_text = \
f"""📍SECTION_EDIT 001 {TOAD_ORIG}
```python
DT = 0.1
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])

    res0 = resp["results"][0]
    assert res0["file_name_edit"] == str(TOAD_ORIG)
    assert res0["file_text"] == text_expected
    print(colored("test01_partial_edit PASSED", "green"))


def test02_partial_edit():
    text_expected = (TEST11_DATA / "toad_partial_edit_02.py").read_text()
    ticket_text = \
f"""📍SECTION_EDIT 001 {TOAD_ORIG}
```python
    def croak(self, x, y, n_times):
        for _ in range(n_times):
            print("croak")
            echo_times = self.calculate_echo_time(x, y)
            for t in echo_times:
                print(f"Echo after {{t:.2f}} seconds")
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])

    res0 = resp["results"][0]
    assert res0["file_name_edit"] == str(TOAD_ORIG), res0
    assert res0["file_text"] == text_expected, print(res0["file_text"])
    print(colored("test02_partial_edit PASSED", "green"))


def test01_rewrite_symbol():
    text_expected = (TEST11_DATA / "toad_rewrite_symbol_01.py").read_text()
    ticket_text = \
f"""📍REPLACE_SYMBOL 001 {TOAD_ORIG} SYMBOL_NAME standalone_jumping_function
```python
def brand_new_function():
    print("I am really a brand new function!")
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])
    res0 = resp["results"][0]
    assert res0["file_name_edit"] == str(TOAD_ORIG), res0
    assert res0["file_text"] == text_expected, res0["file_text"]
    print(colored("test01_rewrite_symbol PASSED", "green"))


def test02_rewrite_symbol():
    text_expected = (TEST11_DATA / "toad_rewrite_symbol_02.py").read_text()
    ticket_text = \
        f"""📍REPLACE_SYMBOL 001 {TOAD_ORIG} SYMBOL_NAME Toad::bounce_off_banks
```python
    def bounce_off_banks(self, pond_width, pond_height):
        pass
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])
    res0 = resp["results"][0]
    assert res0["file_name_edit"] == str(TOAD_ORIG), res0
    assert res0["file_text"] == text_expected, res0["file_text"]
    print(colored("test02_rewrite_symbol PASSED", "green"))


def test03_rewrite_symbol():
    text_expected = (TEST11_DATA / "toad_rewrite_symbol_03.py").read_text()
    ticket_text = \
        f"""📍REPLACE_SYMBOL 001 {TOAD_ORIG} SYMBOL_NAME DT
```python
DT = 10.
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])
    res0 = resp["results"][0]
    assert res0["file_name_edit"] == str(TOAD_ORIG), res0
    assert res0["file_text"] == text_expected, res0["file_text"]
    print(colored("test03_rewrite_symbol PASSED", "green"))


def test04_rewrite_symbol():
    orig_path = (TEST11_DATA / "toad_rewrite_symbol_04_orig.rs").resolve()
    text_expected = (TEST11_DATA / "toad_rewrite_symbol_04_patched.rs").read_text()
    ticket_text = \
        """📍REPLACE_SYMBOL 000 {orig_path} SYMBOL_NAME partition
```rust
fn partition(arr: &mut [i32]) -> usize {
    arr.swap(i, pivot_index);
    i
}
```
"""
    ticket_text = ticket_text.replace("{orig_path}", str(orig_path))
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["000"])
    res0 = resp["results"][0]
    assert res0["file_name_edit"] == str(orig_path), res0
    assert res0["file_text"] == text_expected, res0["file_text"]
    print(colored("test04_rewrite_symbol PASSED", "green"))


def test01_already_applied_rewrite_symbol():
    test_file = TEST11_DATA / "already_applied_rewrite_symbol_01.py"
    ticket_text = \
        f"""📍REPLACE_SYMBOL 001 {test_file} SYMBOL_NAME standalone_jumping_function
```python
def brand_new_function():
    print("I am really a brand new function!")
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])
    assert resp["ticket_ids_already_applied"] == ["001"], resp
    print(colored("test01_already_applied_rewrite_symbol PASSED", "green"))


def test02_already_applied_rewrite_symbol():
    test_file = TEST11_DATA / "already_applied_rewrite_symbol_02.py"
    ticket_text = \
f"""📍REPLACE_SYMBOL 001 {test_file} SYMBOL_NAME Toad::bounce_off_banks
```python
    def bounce_off_banks(self, pond_width, pond_height):
        pass
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])
    assert resp["ticket_ids_already_applied"] == ["001"], resp
    print(colored("test02_already_applied_rewrite_symbol PASSED", "green"))


def test01_already_applied_rewrite_whole_file():
    text_expected = TOAD_ORIG.read_text()
    ticket_text = \
        f"""📍REPLACE_FILE 001 {TOAD_ORIG}
```python
{text_expected}
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])
    assert resp["ticket_ids_already_applied"] == [], resp["ticket_ids_already_applied"]
    print(colored("test02_already_applied_rewrite_symbol PASSED", "green"))


def test01_already_applied_new_file():
    text_expected = TOAD_ORIG.read_text()
    ticket_text = \
        f"""📍NEW_FILE 001 {TOAD_ORIG}
```python
{text_expected}
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])
    assert resp["ticket_ids_already_applied"] == ["001"], resp
    print(colored("test01_already_applied_new_file PASSED", "green"))


def test01_already_fallback_rewrite_symbol():
    text_expected = (TEST11_DATA / "toad_partial_edit_01.py").read_text()
    ticket_text = \
        f"""📍REPLACE_SYMBOL 001 {TOAD_ORIG}
```python
DT = 0.1
```
"""
    messages = make_messages(ticket_text)
    resp = patch_request(messages, ["001"])

    res0 = resp["results"][0]
    assert res0["file_name_edit"] == str(TOAD_ORIG)
    assert res0["file_text"] == text_expected
    print(colored("test01_already_fallback_rewrite_symbol PASSED", "green"))


if __name__ == "__main__":
    test01_rewrite_whole_file()
    test01_new_file()

    test01_rewrite_symbol()
    test02_rewrite_symbol()
    test03_rewrite_symbol()
    test04_rewrite_symbol()

    test01_already_applied_rewrite_symbol()
    test02_already_applied_rewrite_symbol()
    test01_already_applied_rewrite_whole_file()
    test01_already_applied_new_file()

    test01_partial_edit()
    test02_partial_edit()
    test01_already_fallback_rewrite_symbol()

