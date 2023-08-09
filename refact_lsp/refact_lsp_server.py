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


global_only_one_active_request: Optional[Coroutine[Any, Any, Any]] = None
global_socket_session: Optional[aiohttp.ClientSession] = None


@server.feature(TEXT_DOCUMENT_COMPLETION)
async def completions(params: CompletionParams):
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

    try:
        files = {
            short_filename: "\n".join(document.lines),
        }

        current_line = document.lines[params.position.line].strip()
        print("asked for completions \"%s\"" % current_line)
        if current_line.endswith("trigger_text"):
            print(files)
            # completion_coroutine = refact_client.regular_code_completion(
            #     global_socket_session,
            #     files,
            #     "hello_world.py",
            #     len(example1),
            #     50,
            #     multiline=True,
            # )

            items = [
                CompletionItem(
                    label="trigger_text.line1\nline2\nline3",
                    text_edit=TextEdit(
                        range=Range(
                            start=Position(line=params.position.line, character=params.position.character - len("trigger_text")),
                            end=params.position
                        ),
                        new_text="trigger_text.line1\nline2\nline3"
                    )
                )
            ]
            await asyncio.sleep(5)

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
        # Process each content change
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
