import asyncio
import aiohttp
from typing import Optional, List, Tuple
from refact.cmdline_printing import get_terminal_width, tokens_len
import refact.cmdline_main as cmdline_main
from prompt_toolkit.layout.containers import Window, Container
from prompt_toolkit.layout.controls import FormattedTextControl

vecdb_ast_status = {
    "detail": "trying to connect..."
}


black = "ansiblack"
red = "#e8746e"
green = "#6ac496"
text_gray = "#333333"
light_gray = "#3e4957"
gray = "#252b37"
white = "#d4d4d6"


async def update_vecdb_status_background_task():
    global vecdb_ast_status
    while True:
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(f"{cmdline_main.lsp.base_url()}/rag-status") as response:
                    vecdb_ast_status = await response.json(content_type=None)
        except Exception as e:
            print(e)

        if vecdb_ast_status is None:
            await asyncio.sleep(2)
            continue

        vecdb = vecdb_ast_status.get("vecdb", None)
        cmdline_main.app.invalidate()
        if vecdb is not None and vecdb.get("state") == "done":
            await asyncio.sleep(2)
        else:
            await asyncio.sleep(0.1)


def get_percentage(unparsed: int, total: int) -> Optional[str]:
    if total == 0:
        return None
    files_processed = total - unparsed
    percentage = int((files_processed / total) * 100)
    text = f"{files_processed}/{total} ({percentage}%)"
    return text


def status_bar_section_1() -> Tuple[str, str, str]:
    ast = vecdb_ast_status.get("ast", None)
    vecdb = vecdb_ast_status.get("vecdb", None)

    if vecdb is not None:
        if vecdb_state := vecdb.get("state", None):
            if vecdb_state == "parsing":
                percentage = get_percentage(
                    vecdb["files_unprocessed"], vecdb["files_total"])
                if percentage is not None:
                    return (red, text_gray, f"VecDb: {percentage}")
            if vecdb_state != "done":
                return (red, text_gray, f"VecDb: {vecdb_state}")

    if ast is not None:
        if ast_state := ast.get("state", None):
            if ast_state == "indexing":
                percentage = get_percentage(
                    ast["files_unparsed"], ast["files_total"])
                if percentage is not None:
                    return (red, text_gray, f" Ast: {percentage}")
            if ast_state != "done" and ast_state != "idle":
                return (red, text_gray, f"Ast: {ast_state}")

    return (green, text_gray, "Done")


def status_bar_section_2() -> Optional[Tuple[str, str, str]]:
    vecdb = vecdb_ast_status.get("vecdb")
    if vecdb is None:
        return None

    db_size = vecdb.get("db_size", None)
    db_cache_size = vecdb.get("db_cache_size", None)
    text = f"⛁ VecDB Size: {db_size}   VecDB Cache: {db_cache_size}"
    return (light_gray, white, text)


def status_bar_section_3() -> Optional[Tuple[str, str, str]]:
    ast = vecdb_ast_status.get("ast")
    if ast is None:
        return None

    ast_files = ast.get("ast_index_files_total", None)
    ast_symbols = ast.get("ast_index_symbols_total", None)
    text = f"⛁ AST files {ast_files}   AST symbols {ast_symbols}"
    return (gray, white, text)


def create_status_bar(sections: List[Tuple[str, str, str]]) -> List[Tuple[str, str]]:
    result = []

    previous_colour = None

    for (c1, c2, text) in sections:
        if previous_colour is not None:
            result.append((f'{previous_colour} bg:{c1}', ''))
        result.append((f'{c2} bg:{c1}', f" {text} "))
        previous_colour = c1

    # add spaces to the end so the remaining part of the line is filled
    width = get_terminal_width()
    len = tokens_len(result)
    space_len = width - len
    result.append((f"bg:{previous_colour}", " " * space_len))

    return result


def bottom_status_bar():
    # To check data fields use:
    # curl http://127.0.0.1:8001/v1/rag-status

    sections = [
        status_bar_section_1(),
        status_bar_section_2(),
        status_bar_section_3()
    ]

    result = create_status_bar([s for s in sections if s is not None])

    return result


class StatusBar:
    def __init__(self):
        self.formatted_text_control = FormattedTextControl(text=bottom_status_bar)
        self.window = Window(content=self.formatted_text_control, height=1)

    def __pt_container__(self) -> Container:
        return self.window
