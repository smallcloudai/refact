import asyncio
from pygls.server import LanguageServer
from refact_lsp import refact_client


from lsprotocol.types import (
    TEXT_DOCUMENT_COMPLETION,
    CompletionItem,
    CompletionList,
    CompletionParams,
    CompletionItemKind,
    TextEdit,
    Range,
    Position,
)


server = LanguageServer("refact-lsp", "v0.1")


# def run_diff_call(func, src_py, src_txt, cursor, intent):
#     j = inf.nlp_model_call(
#         "contrast",
#         MODEL,
#         sources={src_py: src_txt},
#         intent=intent,
#         function=func,
#         cursor_file=src_py,
#         cursor0=cursor,
#         cursor1=cursor,
#         temperature=TEMPERATURE,
#         max_tokens=MAX_TOKENS,
#         top_p=TOP_P,
#         max_edits=1,
#         verbose=1,
#     )
#     if "status" not in j or j["status"] != "completed":
#         log(str(j))
#         quit(1)
#     return j["choices"][0]


@server.feature(TEXT_DOCUMENT_COMPLETION)
async def completions(params: CompletionParams):
    items = []
    document = server.workspace.get_document(params.text_document.uri)
    current_line = document.lines[params.position.line].strip()
    print("\"%s\"" % current_line)
    if current_line.endswith("hello."):
        items = [
            CompletionItem(label="world"),
            CompletionItem(label="friend"),
        ]
    if current_line.endswith("trigger_text"):
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
        await asyncio.sleep(1)

    return CompletionList(
        is_incomplete=False,
        items=items,
    )


if __name__ == '__main__':
    server.start_tcp("127.0.0.1", 1337)
