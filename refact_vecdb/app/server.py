import uvicorn
from fastapi import FastAPI

from bootstrap import bootstrap
from routers import FindRouter, UploadRouter, DeleteAllRecordsRouter

if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--host", type=str, default="0.0.0.0")
    parser.add_argument("--port", type=int, default=8009)
    args = parser.parse_args()

    # logdir = args.workdir / "logs"
    # logdir.mkdir(exist_ok=True, parents=False)
    # file_handler = logging.FileHandler(filename=logdir / f"server_{datetime.now():%Y-%m-%d-%H-%M-%S}.log")
    # stream_handler = logging.StreamHandler(stream=sys.stdout)
    # logging.basicConfig(level=logging.INFO, handlers=[stream_handler, file_handler])

    app = FastAPI(
        # docs_url=None, redoc_url=None # UNCOMMENT ME ON PROD
    )
    app.include_router(FindRouter())
    app.include_router(UploadRouter())
    app.include_router(DeleteAllRecordsRouter())

    @app.on_event("startup")
    async def startup_event():
        bootstrap(["0.0.0.0"], 9042, "cassandra", "cassandra")
    uvicorn.run(app, host=args.host, port=args.port, loop="uvloop", timeout_keep_alive=600)
