from code_scratchpads import scratchpad_code_completion
from code_scratchpads.scratchpads_code_completion import single_file_fim
from code_scratchpads.cached_tokenizers import cached_get_tokenizer
from code_scratchpads import forward_to_hf_endpoint

from fastapi import FastAPI, APIRouter, HTTPException, Request, Depends
from fastapi.responses import StreamingResponse
from fastapi.param_functions import Query, Optional
from fastapi.security import HTTPBearer
from pydantic import BaseModel
from typing import Dict, List, Union, AsyncGenerator
import logging
import argparse
import uvicorn
import time
import os
import json


logger = logging.getLogger("HTTP")


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


class CompletionsRouter(APIRouter):
    def __init__(self,
        forward_to_hf_endpoint: str,
    ):
        super().__init__()
        self._forward_to_hf_endpoint = forward_to_hf_endpoint
        self.add_api_route("/code-completion", self.code_completion, methods=["POST"])

    async def code_completion(self, post: CodeCompletionCall, bearer: str = Depends(HTTPBearer(auto_error=False))):
        t0 = time.time()
        tokenizer = cached_get_tokenizer(post.model)
        spad: scratchpad_code_completion.ScratchpadCodeCompletion = single_file_fim.SingleFileFIM(
            request_created_ts=t0,
            tokenizer=tokenizer,
            max_new_tokens=post.parameters.max_new_tokens,
            **_validate_code_completion_parameters(post.inputs)
        )
        sampling_parameters = post.parameters.dict(exclude_unset=True)
        sampling_parameters["return_full_text"] = False
        t1 = time.time()
        prompt = spad.prompt(2048, sampling_parameters_to_patch=sampling_parameters)
        t2 = time.time()
        text_generator: AsyncGenerator[str, None] = forward_to_hf_endpoint.real_work(
            model_name=post.model,
            prompt=prompt,
            sampling_parameters=sampling_parameters,
            stream=post.stream,
            auth_from_client=(bearer.credentials if bearer else None),
        )
        re_stream = spad.re_stream_response(text_generator)
        logger.info("code-completion init+tokenizer %0.2fms, prompt %0.2fms" % (1000*(t1-t0), 1000*(t2-t1)))
        return StreamingResponse(code_completion_streamer(
            re_stream,
            request_created_ts=t0,
            real_stream=post.stream,
            ))


async def code_completion_streamer(re_stream, request_created_ts, real_stream):
    scratchpad_says: List[str]
    async for scratchpad_says in re_stream:
        if not real_stream:
            yield json.dumps(scratchpad_says)
            return
        tmp = json.dumps(scratchpad_says)
        yield "data: " + tmp + "\n\n"
    yield "data: [DONE]" + "\n\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--forward-to-hf-endpoint", type=str, help="Forward to this endpoint")
    args = parser.parse_args()

    app = FastAPI(title="Code Completion", description="Code Completion for Python")
    app.include_router(CompletionsRouter(forward_to_hf_endpoint=args.forward_to_hf_endpoint))

    DEBUG = int(os.environ.get("DEBUG", "0"))
    # Startup event of FastAPI
    uvicorn_logger = logging.getLogger("uvicorn.error")

    logging.basicConfig(
        level=logging.DEBUG if DEBUG else logging.INFO,
        format='%(asctime)s %(message)s',
        datefmt='%Y%m%d %H:%M:%S'
    )

    @app.on_event("shutdown")
    def startup_event():
        if args.forward_to_hf_endpoint:
            forward_to_hf_endpoint.global_hf_session_close()

    uvicorn.run(app,
        workers=1,
        host="127.0.0.1",
        port=8008,
        log_config=None,
    )
