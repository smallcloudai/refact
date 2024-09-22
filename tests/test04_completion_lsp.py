
from termcolor import colored

from lsp_connect import LSPConnectOptions, LSPCall


def lsp_completion():
    file_name = "hello.py"
    hello_py = "def this_function_returns_hello_world():\n    "

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
        # On shutdown, visible errors are "Unexpected params: {}", not much we can do about this, it's the pylspclient library that sends the shutdown.
    completion = cc["choices"][0]["code_completion"]
    print()
    print("%s%s" % (
        colored(hello_py, "white"),
        colored(completion, "green")
    ))
    assert completion in ["return \"Hello World!\"", "return \"Hello World\""]


if __name__ == '__main__':
    lsp_completion()
