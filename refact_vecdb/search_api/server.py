from argparse import ArgumentParser

import uvicorn

from fastapi import FastAPI

from refact_vecdb.common.context import ACCOUNT
from refact_vecdb.search_api.bootstrap import bootstrap
from refact_vecdb.search_api.routers import MainRouter


__all__ = ["main"]


def main():
    parser = ArgumentParser()
    parser.add_argument("--host", type=str, default="0.0.0.0")
    parser.add_argument("--port", type=int, default=8883)
    parser.add_argument('--cassandra_host', type=str, default="10.190.99.200")
    parser.add_argument('--cassandra_port', type=int, default=9042)
    args = parser.parse_args()

    app = FastAPI(
        # docs_url=None, redoc_url=None
    )

    app.include_router(MainRouter())
    bootstrap(ACCOUNT, args.cassandra_host, args.cassandra_port)

    uvicorn.run(app, host=args.host, port=args.port, loop="uvloop", timeout_keep_alive=600)


if __name__ == "__main__":
    main()
