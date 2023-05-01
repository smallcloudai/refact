import logging
import asyncio
import sys

from hypercorn.config import Config
from hypercorn.asyncio import serve

from datetime import datetime
from pathlib import Path
from fastapi import FastAPI

from refact_self_hosting.gen_certificate import gen_certificate
from refact_self_hosting.inference import Inference
from refact_self_hosting.routers import ActivateRouter
from refact_self_hosting.routers import CompletionRouter
from refact_self_hosting.routers import ContrastRouter


if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--host", type=str, default="0.0.0.0")
    parser.add_argument("--port", type=int, default=8008)
    parser.add_argument("--cpu", action="store_true")
    parser.add_argument("--workdir", type=Path)
    parser.add_argument("--model", type=str)
    args = parser.parse_args()

    logdir = args.workdir / "logs"
    logdir.mkdir(exist_ok=True, parents=False)
    file_handler = logging.FileHandler(filename=logdir / f"server_{datetime.now():%Y-%m-%d-%H-%M-%S}.log")
    stream_handler = logging.StreamHandler(stream=sys.stdout)
    logging.basicConfig(level=logging.INFO, handlers=[stream_handler, file_handler])

    inference = Inference(workdir=args.workdir, model_name=args.model, force_cpu=args.cpu)

    app = FastAPI(docs_url=None)
    app.include_router(CompletionRouter(inference))
    app.include_router(ContrastRouter(inference))

    key_filename, cert_filename = gen_certificate(args.workdir)

    config = Config()
    config.bind = f"{args.host}:{args.port}"
    config.accesslog = "-"
    config.keyfile = key_filename
    config.certfile = cert_filename
    config.keep_alive_timeout = 600
    config.graceful_timeout = 600

    asyncio.run(serve(app=app, config=config))
