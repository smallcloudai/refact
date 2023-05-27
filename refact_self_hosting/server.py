import logging
import asyncio
import sys

from datetime import datetime
from pathlib import Path
from fastapi import FastAPI

import uvicorn

from refact_self_hosting.gen_certificate import gen_certificate
from refact_self_hosting.inference import Inference
from refact_self_hosting.routers import LongthinkFunctionGetterRouter
from refact_self_hosting.routers import CompletionRouter
from refact_self_hosting.routers import ContrastRouter
from refact_self_hosting.routers import ChatRouter


if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--host", type=str, default="0.0.0.0")
    parser.add_argument("--port", type=int, default=8008)
    parser.add_argument("--cpu", action="store_true")
    parser.add_argument("--workdir", type=Path)
    parser.add_argument("--model", type=str)
    parser.add_argument("--finetune", type=str)
    args = parser.parse_args()

    logdir = args.workdir / "logs"
    logdir.mkdir(exist_ok=True, parents=False)
    file_handler = logging.FileHandler(filename=logdir / f"server_{datetime.now():%Y-%m-%d-%H-%M-%S}.log")
    stream_handler = logging.StreamHandler(stream=sys.stdout)
    logging.basicConfig(level=logging.INFO, handlers=[stream_handler, file_handler])

    inference = Inference(force_cpu=args.cpu)

    app = FastAPI(docs_url=None)
    app.include_router(CompletionRouter(inference))
    app.include_router(ContrastRouter(inference))
    app.include_router(LongthinkFunctionGetterRouter(inference))
    app.include_router(ChatRouter(inference))

    @app.on_event("startup")
    async def startup_event():
        asyncio.create_task(inference.model_setup_loop_forever(
            model_name=args.model, workdir=args.workdir, finetune=args.finetune
        ))

    key_filename, cert_filename = gen_certificate(args.workdir)
    uvicorn.run(
        app, host=args.host, port=args.port,
        loop="uvloop", timeout_keep_alive=600,
        ssl_keyfile=key_filename, ssl_certfile=cert_filename)
