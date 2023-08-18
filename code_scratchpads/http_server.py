from code_scratchpads import scratchpad_code_completion
from code_scratchpads.scratchpads_code_completion import single_file_fim
from code_scratchpads.cached_tokenizers import cached_get_tokenizer

from fastapi import FastAPI, APIRouter, HTTPException, Request
from fastapi.param_functions import Query, Optional
from pydantic import BaseModel
from typing import Dict, List, Union
import logging
import argparse
import uvicorn
import time
import os


logger = logging.getLogger("AAA")


FILE_TOO_BIG = 200_000


forward_to_hf_endpoint = ""


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
    model: str = Query(default="", pattern="^[a-z/A-Z0-9_\.]+$")
    inputs: CodeCompletionTask
    parameters: SamplingParameters
    stream: bool = False



def _validate_code_completion_parameters(task: CodeCompletionTask):
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
    if task.cursor.line > len(cursor_source_split):
        raise HTTPException(status_code=400, detail="cursor line=%d is beyond file length=%d" % (task.cursor.line, len(cursor_source_split)))
    if task.cursor.character > len(cursor_source_split[task.cursor.line]):
        raise HTTPException(status_code=400, detail="cursor character=%d is beyond line %d length=%d" % (task.cursor.character, task.cursor.line, len(cursor_source_split[task.cursor.line])))
    return {
        "sources": sources_split,
        "cursor_file": task.cursor.file,
        "cursor_line": task.cursor.line,
        "cursor_character": task.cursor.character,
        "multiline": task.multiline,
    }


class CompletionsRouter(APIRouter):
    def __init__(self,
        forward_to_hf_endpoint: str,
    ):
        super().__init__()
        self._forward_to_hf_endpoint = forward_to_hf_endpoint
        self.add_api_route("/code-completion", self.code_completion, methods=["POST"])

    async def code_completion(self, post: CodeCompletionCall, request: Request):
        t0 = time.time()
        tokenizer = cached_get_tokenizer(post.model)
        spad: scratchpad_code_completion.ScratchpadCodeCompletion = single_file_fim.SingleFileFIM(
            request_created_ts=t0,
            tokenizer=tokenizer,
            max_new_tokens=post.parameters.max_new_tokens,
            **_validate_code_completion_parameters(post.inputs)
        )
        t1 = time.time()
        prompt = spad.prompt(2048)
        t2 = time.time()
        logger.info("code-completion init+tokenizer %0.2fms, prompt %0.2fms" % (1000*(t1-t0), 1000*(t2-t1)))
        return True


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--forward-to-hf-endpoint", type=str, help="Forward to this endpoint")
    args = parser.parse_args()

    app = FastAPI(title="Code Completion", description="Code Completion for Python")
    app.include_router(CompletionsRouter(forward_to_hf_endpoint=args.forward_to_hf_endpoint))

    DEBUG = int(os.environ.get("DEBUG", "0"))
    logging.basicConfig(level=logging.DEBUG if DEBUG else logging.INFO)

    uvicorn.run(app,
        workers=1,
        host="127.0.0.1",
        port=8008,
        # debug=True,
        # loop="asyncio",
    )
