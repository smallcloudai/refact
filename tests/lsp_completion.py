
from termcolor import colored

from lsp_connect import LSPConnectOptions, LSPCall


def lsp_completion():
    file_name = "hello.py"
    hello_py = "def hello_world():\n    "

    with LSPCall(LSPConnectOptions()) as lsp:
        lsp.load_document(file_name, hello_py)

        cc = lsp.get_completions(
            file_name,
            pos=(1, 4),
            multiline=False,
            params={
                "max_new_tokens": 20,
                "temperature": 0.1
            },
        )
    print("%s%s" % (
        colored(hello_py, "green"),
        colored(cc["choices"][0]["code_completion"], "magenta")
    ))


if __name__ == '__main__':
    lsp_completion()
