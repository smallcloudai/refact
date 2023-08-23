from fastapi import HTTPException
from fastapi.param_functions import Query, Optional
from pydantic import BaseModel
from typing import Dict, List


FILE_TOO_BIG = 200_000


class Position(BaseModel):
    file: str
    line: int         # zero based, names like in LSP
    character: int


class CodeCompletionTask(BaseModel):
    sources: Dict[str, str]
    cursor: Position
    multiline: bool = False


class SamplingParameters(BaseModel):
    max_new_tokens: int = Query(default=50, ge=0, le=4096)
    temperature: Optional[float] = Query(default=None, ge=0.0, le=2.0)
    top_p: Optional[float] = Query(default=None, ge=0.5, le=1.0)
    stop: Optional[List[str]] = Query(default=None, min_items=0, max_items=10)


class CodeCompletionCall(BaseModel):
    model: str = Query(default="", pattern="^[a-z/A-Z0-9_\.]*$")
    inputs: CodeCompletionTask
    parameters: SamplingParameters
    stream: bool = False


def validate_code_completion_parameters(task: CodeCompletionTask):
    if task.cursor.file not in task.sources:
        raise HTTPException(status_code=400, detail="cursor.file='%s' is not in sources=%s" % (task.cursor.file, list(task.sources.keys())))
    if task.cursor.line < 0 or task.cursor.character < 0:
        raise HTTPException(status_code=400, detail="cursor position is negative (%d, %d)" % (task.cursor.line, task.cursor.character))
    sources_split: Dict[str, List[str]] = {}
    for fn, text in task.sources.items():
        if len(text) > FILE_TOO_BIG:
            raise HTTPException(status_code=400, detail="file '%s' is too long (%d bytes)" % (fn, len(text)))
        sources_split[fn] = text.splitlines()
    cursor_source_split = sources_split[task.cursor.file]
    lines_count = len(cursor_source_split)
    if task.cursor.line > lines_count:
        raise HTTPException(status_code=400, detail="cursor line=%d is beyond file length=%d" % (task.cursor.line, len(cursor_source_split)))
    if task.cursor.line < lines_count:
        if task.cursor.character > len(cursor_source_split[task.cursor.line]):
            raise HTTPException(status_code=400, detail="cursor character=%d is beyond line %d length=%d" % (task.cursor.character, task.cursor.line, len(cursor_source_split[task.cursor.line])))
    else:
        if task.cursor.character > 0:
            raise HTTPException(status_code=400, detail="cursor character=%d is beyond end of file" % (task.cursor.character))
    return {
        "sources": sources_split,
        "cursor_file": task.cursor.file,
        "cursor_line": task.cursor.line,
        "cursor_character": task.cursor.character,
        "multiline": task.multiline,
    }
