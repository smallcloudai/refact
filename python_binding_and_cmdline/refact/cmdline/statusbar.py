import asyncio
import aiohttp
from refact.cmdline.printing import get_terminal_width, tokens_len
import refact.cmdline.main as cmdline_main
from prompt_toolkit.layout.containers import Window, Container
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.application import get_app_or_none


vecdb_ast_status = {
    "detail": "Initializing..."
}

model_section = ""


async def statusbar_background_task():
    global vecdb_ast_status
    while get_app_or_none() is not None:
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(f"{cmdline_main.lsp.base_url()}/rag-status") as response:
                    vecdb_ast_status = await response.json(content_type=None)
        except Exception as e:
            if get_app_or_none() is not None:
                print(e)

        if vecdb_ast_status is None:
            await asyncio.sleep(2)
            continue

        if get_app_or_none() is None:
            return
        cmdline_main.app.invalidate()

        fast_sleep = False
        if ast := vecdb_ast_status.get("ast", None):
            if ast.get("state") == "indexing":
                fast_sleep = True
        if ast := vecdb_ast_status.get("vecdb", None):
            if ast.get("state") == "parsing":
                fast_sleep = True
        if fast_sleep:
            await asyncio.sleep(0.1)
        else:
            await asyncio.sleep(2)


def bottom_status_bar():
    # To check data fields use:
    # curl http://127.0.0.1:8001/v1/rag-status

    ast_text = "⛁ AST off"
    vdb_text = "⛁ VecDB off"
    ast_color = "#fac496"
    vdb_color = "#fac496"

    if ast := vecdb_ast_status.get("ast", None):
        if ast["state"] == "indexing":
            ast_parsed_qty = ast["files_total"] - ast["files_unparsed"]
            ast_text = "⛁ AST parsing %4d/%d" % (ast_parsed_qty, ast["files_total"])
        elif ast["state"] == "starting":
            ast_text = "⛁ AST starting"
        elif ast["state"] == "done":
            ast_text = "⛁ AST %d files %d symbols" % (ast["ast_index_files_total"], ast["ast_index_symbols_total"])
            ast_color = "#A0FFA0"

    if vecdb := vecdb_ast_status.get("vecdb", None):
        if vecdb["state"] not in ["done", "idle"]:
            vecdb_parsed_qty = vecdb["files_total"] - vecdb["files_unprocessed"]
            vdb_text = "⛁ VecDB vectorizing %4d/%d files" % (vecdb_parsed_qty, vecdb["files_total"])
        else:
            vdb_text = "⛁ VecDB %d records" % (vecdb["db_size"])
            vdb_color = "#A0FFA0"

    sections = [
        (ast_color, '#121212', "%-35s" % ast_text),
        (vdb_color, '#121221', "%-35s" % vdb_text),
        ("#A0FFA0", '#121212', "%-35s" % model_section),
    ]

    result = []
    previous_colour = None

    for (c1, c2, text) in sections:
        if previous_colour is not None:
            result.append((f'{previous_colour} bg:#101010', '║'))
        result.append((f'{c2} bg:{c1}', f" {text} "))
        previous_colour = c1

    # remaining part of the line
    width = get_terminal_width()
    len = tokens_len(result)
    space_len = width - len
    if previous_colour:
        result.append((f"bg:{previous_colour}", " " * space_len))

    return result


class StatusBar:
    def __init__(self):
        self.formatted_text_control = FormattedTextControl(text=bottom_status_bar)
        self.window = Window(content=self.formatted_text_control, height=1)

    def __pt_container__(self) -> Container:
        return self.window
