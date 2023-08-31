from multiprocessing import Manager
from argparse import ArgumentParser

import uvicorn
from fastapi import FastAPI

from refact_vecdb.embeds_api.routers import MainRouter
from refact_vecdb.embeds_api.context import CONTEXT as C
from refact_vecdb.embeds_api.bootstrap import bootstrap


__all__ = ['main']


def main():
    parser = ArgumentParser()
    parser.add_argument("--host", type=str, default="0.0.0.0")
    parser.add_argument("--port", type=int, default=8882)
    args = parser.parse_args()

    app = FastAPI()
    with Manager() as q_manager:
        C.q_manager = q_manager
        app.include_router(MainRouter())
        bootstrap()
        uvicorn.run(app, host=args.host, port=args.port, loop='uvloop')


if __name__ == '__main__':
    main()

