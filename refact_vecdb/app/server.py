import uvicorn

from fastapi import FastAPI

from refact_vecdb.app.bootstrap import bootstrap
from refact_vecdb.app.routers import StatusRouter, FindRouter, UploadRouter, DeleteAllRecordsRouter
from refact_vecdb.app.embed_spads import embed_providers


if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--provider", type=str, default="gte", choices=embed_providers.keys())
    parser.add_argument("--host", type=str, default="0.0.0.0")
    parser.add_argument("--port", type=int, default=8009)
    parser.add_argument('--cassandra_host', type=str, default="0.0.0.0")
    parser.add_argument('--cassandra_port', type=int, default=9042)
    args = parser.parse_args()

    # logdir = args.workdir / "logs"
    # logdir.mkdir(exist_ok=True, parents=False)
    # file_handler = logging.FileHandler(filename=logdir / f"server_{datetime.now():%Y-%m-%d-%H-%M-%S}.log")
    # stream_handler = logging.StreamHandler(stream=sys.stdout)
    # logging.basicConfig(level=logging.INFO, handlers=[stream_handler, file_handler]) 

    app = FastAPI(docs_url=None, redoc_url=None)

    app.include_router(StatusRouter())
    app.include_router(FindRouter())
    app.include_router(UploadRouter())
    app.include_router(DeleteAllRecordsRouter())

    @app.on_event("startup")
    async def startup_event():
        bootstrap(
            args.provider,
            args.cassandra_host,
            args.cassandra_port,
            "cassandra",
            "cassandra",
        )
    uvicorn.run(app, host=args.host, port=args.port, loop="uvloop", timeout_keep_alive=600)
