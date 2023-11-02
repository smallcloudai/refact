import re
import difflib

from typing import Optional

from termcolor import colored

from lsp_connect import LSPConnectOptions, LSPCall


class TestReturnAddedTextLSPCall(LSPCall):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    def test_if_head_tail_equal_return_added_text(
            self,
            text_a: str,
            text_b: str,
            orig_grey_text: str
    ):
        resp = self._lsp_client.lsp_endpoint.call_method(
            "refact/test_if_head_tail_equal_return_added_text",
            text_a=text_a,
            text_b=text_b,
            orig_grey_text=orig_grey_text,
        )
        return resp['grey_corrected'], resp['is_valid'], resp['unchanged_percentage']


def _report_test_status(test_name: str, detailed: Optional[str] = None, success: bool = True):
    msg = f""
    if success:
        msg += f"===== {test_name}: SUCCESS ====="
        color = 'green'
    else:
        msg += f"===== {test_name}: FAIL ===== "
        color = 'red'
    if detailed:
        msg += f"\nDETAILED:\n{detailed}"
        msg += "\n=============="

    print(colored(msg, color))

def _check(grey_corrected, grey_corrected_expected, is_valid, is_valid_expected, nvm_spaces: bool = True):
    detailed = ""

    def unsuccessfull_details(text_a, text_b, detailed):
        d = difflib.Differ()
        diff = list(d.compare(grey_corrected.splitlines(), grey_corrected_expected.splitlines()))
        for l in diff:
            if not l.startswith("+") and not l.startswith("-"):
                l = "= " + l
            detailed += l + "\n"
        return detailed

    nvm_spaces_worked = False

    if is_valid != is_valid_expected:
        success = False
        detailed = f"is_valid != is_valid_expected:\n{is_valid}!={is_valid_expected}"

    elif grey_corrected == grey_corrected_expected:
        success = True

    elif nvm_spaces and re.sub(r'\s', '', grey_corrected) == re.sub(r'\s', '', grey_corrected_expected):
        success = True
        nvm_spaces_worked = True
        detailed = "WARNING: only worked with nvm_spaces=True\n"
        detailed = unsuccessfull_details(grey_corrected, grey_corrected_expected, detailed)

    else:
        success = False
        detailed = f"grey_corrected != grey_corrected_expected, nvm_spaces was {nvm_spaces}\n"
        detailed = unsuccessfull_details(grey_corrected, grey_corrected_expected, detailed)

    return {"success": success, "detailed": detailed}

def _test_0():
    test_name = "TEST 0"
    text_a = "def "
    text_b = "def hello_world()"
    orig_grey_text = "hello_world"
    grey_corrected_expected = "hello_world()"
    is_valid_expected = True

    grey_corrected, is_valid, unchanged_percentage = lsp.test_if_head_tail_equal_return_added_text(
        text_a, text_b, orig_grey_text
    )
    print(colored("unchanged_percentage %0.2f" % unchanged_percentage, 'red'))
    _report_test_status(test_name, **_check(
        grey_corrected, grey_corrected_expected, is_valid, is_valid_expected
    ))

def _test_1():
    test_name = "TEST 1"
    text_a = """

def hello_world():
            """
    text_b = """

def hello_world():
    print("Hello World")"""
    orig_grey_text = '    print("Hello World")'
    grey_corrected_expected = '    print("Hello World")'
    is_valid_expected = True

    grey_corrected, is_valid, unchanged_percentage = lsp.test_if_head_tail_equal_return_added_text(
        text_a, text_b, orig_grey_text
    )
    print(colored("unchanged_percentage %0.2f" % unchanged_percentage, 'red'))
    _report_test_status(test_name, **_check(
        grey_corrected, grey_corrected_expected, is_valid, is_valid_expected
    ))

def _test_2():
    test_name = "TEST 2"
    text_a = """
fn _common_characters_in_strings(a: &String, b: &String) -> i64 {
    let diff = TextDiff::from_chars(a, b);
    let mut common = 0;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {}
            ChangeTag::Insert => {}
        }
    }
    common as i64
}
"""
    text_b = """
fn _common_characters_in_strings(a: &String, b: &String) -> i64 {
    let diff = TextDiff::from_chars(a, b);
    let mut common = 0;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {}
            ChangeTag::Insert => {}
            ChangeTag::Equal => {
                common += 1
            }
        }
    }
    common as i64
}
"""
    orig_grey_text = """ChangeTag::Equal => {
                common += 1
            }"""

    grey_corrected_expected = """ChangeTag::Equal => {
                common += 1
            }
    """
    is_valid_expected = True

    grey_corrected, is_valid, unchanged_percentage = lsp.test_if_head_tail_equal_return_added_text(
        text_a, text_b, orig_grey_text
    )
    print(colored("unchanged_percentage %0.2f" % unchanged_percentage, 'red'))
    # success = (grey_corrected == grey_corrected_expected) and (is_valid == is_valid_expected)
    _report_test_status(test_name, **_check(
        grey_corrected, grey_corrected_expected, is_valid, is_valid_expected
    ))


def test_all():
    _test_0()
    _test_1()
    _test_2()


if __name__ == "__main__":
    with TestReturnAddedTextLSPCall(LSPConnectOptions()) as lsp:
        test_all()
