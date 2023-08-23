from code_scratchpads import scratchpad_code_completion
from code_scratchpads.models_db import db_code_completion_models
from code_scratchpads.cached_tokenizers import cached_get_tokenizer
from code_scratchpads import forward_to_hf_endpoint
from code_scratchpads import call_validation

from fastapi import FastAPI, APIRouter, HTTPException, Request, Depends
from fastapi.responses import StreamingResponse
from fastapi.security import HTTPBearer
from typing import AsyncGenerator, Dict, Any, List, Union
import logging
import argparse
import uvicorn
import time
import os
import json
import importlib


logger = logging.getLogger("HTTP")


class CompletionsRouter(APIRouter):
    def __init__(self,
        forward_to_hf_endpoint: bool,
    ):
        super().__init__()
        self._forward_to_hf_endpoint = forward_to_hf_endpoint
        self.add_api_route("/v1/code-completion", self.code_completion, methods=["POST"])
        self.add_api_route("/v1/login", self._login, methods=["GET"])

    async def _login(self, request: Request, bearer: str = Depends(HTTPBearer(auto_error=False))):
        if bearer is None:
            raise HTTPException(status_code=401, detail="No API key provided")
        logger.info("Login from %s, API key ***%s" % (request.client.host, bearer.credentials[-3:]))
        return {
            "account": "dummy_accout",
            "retcode": "OK",
        }

    async def code_completion(self, post: call_validation.CodeCompletionCall, bearer: str = Depends(HTTPBearer(auto_error=False))):
        t0 = time.time()
        model_rec: db_code_completion_models.CompletionModelRecord = db_code_completion_models.model_lookup(post.model)
        if model_rec is None:
            raise HTTPException(status_code=400, detail="model '%s' is not supported" % post.model)
        tokenizer = cached_get_tokenizer(model_rec.model_name)
        module_name, Class_name = model_rec.code_completion_scratchpad.split(":")
        ScratchpadClass = importlib.import_module("code_scratchpads.scratchpads_code_completion." + module_name).__dict__[Class_name]
        assert issubclass(ScratchpadClass, scratchpad_code_completion.ScratchpadCodeCompletion)
        spad: scratchpad_code_completion.ScratchpadCodeCompletion = ScratchpadClass(
            request_created_ts=t0,
            tokenizer=tokenizer,
            max_new_tokens=post.parameters.max_new_tokens,
            supports_stop=model_rec.supports_stop,
            **call_validation.validate_code_completion_parameters(post.inputs)
        )
        sampling_parameters = post.parameters.dict(exclude_unset=True)
        sampling_parameters["return_full_text"] = False
        t1 = time.time()
        prompt = spad.prompt(2048, sampling_parameters_to_patch=sampling_parameters)
        t2 = time.time()
        text_generator: AsyncGenerator[str, None]
        if self._forward_to_hf_endpoint:
            text_generator = forward_to_hf_endpoint.real_work(
                model_name=model_rec.model_name,
                prompt=prompt,
                sampling_parameters=sampling_parameters,
                stream=post.stream,
                auth_from_client=(bearer.credentials if bearer else None),
            )
        else:
            # TODO: alternatives to forward_to_hf_endpoint, such as regular local inference
            pass
        re_stream = spad.re_stream_response(text_generator)
        logger.info("start code-completion model='%s' init+tokenizer %0.2fms, prompt %0.2fms" % (
            model_rec.model_name,
            1000*(t1-t0), 1000*(t2-t1)))
        return StreamingResponse(code_completion_streamer(
            re_stream,
            request_created_ts=t0,
            streaming=post.stream,
            model_name=model_rec.model_name,
            ))


async def code_completion_streamer(re_stream_generator, request_created_ts, model_name, streaming):
    scratchpad_says: Union[Dict[str, Any], List[Dict[str, Any]]]
    try:
        async for scratchpad_says in re_stream_generator:
            if not streaming:
                # list of dicts
                for x in scratchpad_says:
                    x["model"] = model_name
                yield json.dumps(scratchpad_says)
                logger.info("finished request in %0.2fms" % (1000*(time.time()-request_created_ts)))
                return
            else:
                # dict
                scratchpad_says["model"] = model_name
                tmp = json.dumps(scratchpad_says)
                yield "data: " + tmp + "\n\n"
        if streaming:
            logger.info("finished streaming in %0.2fms" % (1000*(time.time()-request_created_ts)))
            yield "data: [DONE]" + "\n\n"
    except ValueError as e:
        # ValueError is a way to stop generation and send message to the user.
        # Message must be a correct json.
        logger.info("returning error json: %s" % e)
        if not streaming:
            yield str(e)
        else:
            yield "data: " + str(e) + "\n\n"
            yield "data: [DONE]" + "\n\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--forward-to-hf-endpoint", type=bool, default=True)
    args = parser.parse_args()

    app = FastAPI(title="Code Completion", description="Code Completion for Python")
    app.include_router(CompletionsRouter(forward_to_hf_endpoint=args.forward_to_hf_endpoint))

    DEBUG = int(os.environ.get("DEBUG", "0"))
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
        host="0.0.0.0",
        port=8001,
        log_config=None,
    )
