import asyncio
import os
from pygls.server import LanguageServer
from refact_lsp import refact_client
from typing import Any, Coroutine, Optional
import aiohttp


from lsprotocol.types import (
    TEXT_DOCUMENT_COMPLETION,
    TEXT_DOCUMENT_DID_CHANGE,
    CANCEL_REQUEST,
    InitializeParams,
    InitializeResult,
    ServerCapabilities,
    TextDocumentSyncOptions,
    TextDocumentSyncKind,
    CancelRequestNotification,
    InsertTextMode,
    DidChangeTextDocumentParams,
    CompletionItem,
    CompletionList,
    CompletionParams,
    CompletionItemKind,
    CompletionOptions,
    CancelParams,
    TextEdit,
    Range,
    Position,
)


server = LanguageServer("refact-lsp", "v0.1")


global_socket_session: Optional[aiohttp.ClientSession] = None


@server.feature(TEXT_DOCUMENT_COMPLETION)
async def completions(params: CompletionParams):
    global global_socket_session
    if global_socket_session is None:
        global_socket_session = aiohttp.ClientSession(headers={
        "Authorization": "Bearer %s" % os.environ["SMALLCLOUD_API_KEY"],
        })

    items = []
    document = server.workspace.get_document(params.text_document.uri)
    uri = params.text_document.uri
    root_uri = server.workspace.root_uri
    if uri.startswith(root_uri):
        short_filename = uri[len(root_uri):].lstrip("/")
    else:
        short_filename = os.path.basename(uri)
    print("file \"%s\"" % uri)
    print("root_uri \"%s\"" % root_uri)
    print("short path \"%s\"" % short_filename)

    cursor_pos = 0
    for line, line_txt in enumerate(document.lines):
        if params.position.line == line:
            cursor_pos += params.position.character
            break
        cursor_pos += len(line_txt)
    try:
        files = {
            short_filename: "".join(document.lines),
        }
        print("%s|" % files[short_filename][:cursor_pos])

        current_line = document.lines[params.position.line]
        current_line0 = current_line[:params.position.character]
        current_line1 = current_line[params.position.character:]
        print("asked for completions \"%s\" | \"%s\"" % (current_line0.replace("\n", "\\n"), current_line1.replace("\n", "\\n")))
        unfinished_token = ""
        import re
        tmp = current_line0
        while len(tmp) > 0 and re.match(r"\w", tmp[-1]):
            unfinished_token = tmp[-1] + unfinished_token
            tmp = tmp[:-1]
        print("unfinished_token \"%s\"" % unfinished_token)
        unfinished_token_len = len(unfinished_token)
        if 1:
            # print(files)
            # refact_client.global_only_one_active_request.close()
            completion = ""
            try:
                completion = await asyncio.wait_for(
                    refact_client.regular_code_completion(
                        global_socket_session,
                        files,
                        short_filename,
                        cursor_pos,
                        50,
                        multiline=True,
                    ),
                    timeout=30,
                )
                print("completion success \"%s\"" % completion.replace("\n", "\\n"))
            except Exception as e:
                print("exception %s (%s)\n\n" % (e, str(type(e))))

            items = []
            if completion:
                label = completion.rstrip("\n")
                if "\n" in label:
                    tmp = label.split("\n")
                    label = tmp[0] + " → %d lines more" % (len(tmp) - 1)
                items = [
                    # CompletionItem(label="hello", kind=CompletionItemKind.Text),
                    # CompletionItem(label="world"),
                    CompletionItem(
                        label=unfinished_token + label,
                        kind=CompletionItemKind.Text,
                        text_edit=TextEdit(
                            range=Range(
                                # start=Position(line=params.position.line, character=params.position.character - unfinished_token_len),
                                # end=params.position,
                                start=Position(line=params.position.line, character=params.position.character - unfinished_token_len),
                                end=Position(line=params.position.line, character=len(current_line)),
                            ),
                            new_text=(unfinished_token + completion.rstrip("\n")),
                        ),
                        insert_text_mode=InsertTextMode.AsIs,
                    ),
                ]

    except asyncio.CancelledError:
        print("Cancelled")
    finally:
        print("finally", current_line.strip())

    return CompletionList(
        is_incomplete=False,
        items=items,
    )


@server.feature(TEXT_DOCUMENT_DID_CHANGE)
async def did_change(params: DidChangeTextDocumentParams):
    for change in params.content_changes:
        range = change.range
        text = change.text
        print("change is \"%s\"" % text)
    return None


@server.feature(CANCEL_REQUEST)
async def cancel_request(params: CancelParams):
    print("Cancel request (1) received", params.id)


# server.register_capability(CANCEL_REQUEST)
# @server.notification(CancelRequestNotification)
# async def handle_cancel_request(params: CancelParams):
#     print("Cancel request (2) received", params.id)
