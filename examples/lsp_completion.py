# pip install pylspclient
import pylspclient
import argparse
import socket
import os
import time
import termcolor


hello_py = "def hello_world():\n    "


def main():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(("127.0.0.1", 8002))
    pipein, pipeout = s.makefile("wb", buffering=0), s.makefile("rb", buffering=0)
    json_rpc_endpoint = pylspclient.JsonRpcEndpoint(pipein, pipeout)
    lsp_endpoint = pylspclient.LspEndpoint(json_rpc_endpoint)
    lsp_client = pylspclient.LspClient(lsp_endpoint)
    capabilities = {}
    root_uri = 'file:///workspace'
    workspace_folders = [{'name': 'workspace', 'uri': root_uri}]
    lsp_client.initialize(1337, None, root_uri, None, capabilities, "off", workspace_folders)
    uri = "file:///workspace/hello.py"
    languageId = pylspclient.lsp_structs.LANGUAGE_IDENTIFIER.PYTHON
    lsp_client.didOpen(pylspclient.lsp_structs.TextDocumentItem(uri, languageId, version=1, text=hello_py))

    cc = lsp_client.lsp_endpoint.call_method(
        "refact/getCompletions",
        textDocument=pylspclient.lsp_structs.TextDocumentIdentifier(uri),
        position=pylspclient.lsp_structs.Position(1, 4),
        parameters={
            "max_new_tokens": 20,
            "temperature": 0.1
        },
        multiline=False
    )
    print("CC result:", cc)
    try:
        # pylspclient conflicts with tower-lsp, shutdown() below sends {} as parameters and lsp-tower expects something else
        # the result is:
        #    {'jsonrpc': '2.0', 'error': {'code': -32602, 'message': 'Unexpected params: {}'}, 'id': 1}
        # printed in console.
        lsp_client.shutdown()    # this has lsp_endpoint.stop() inside, lsp_endpoint is a thread
    except Exception:
        pass
    lsp_endpoint.join()

    print("%s%s" % (
        termcolor.colored(hello_py, "green"),
        termcolor.colored(cc["choices"][0]["code_completion"], "magenta")
    ))


if __name__ == "__main__":
    main()

