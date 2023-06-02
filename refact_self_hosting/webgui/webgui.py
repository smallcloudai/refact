import logging
import asyncio
import uvloop
import sys
import datetime
import signal
import uvicorn
asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from refact_self_hosting.webgui import selfhost_static
from refact_self_hosting.webgui import selfhost_fastapi_completions
from refact_self_hosting.webgui import selfhost_fastapi_gpu
from refact_self_hosting.webgui import tab_upload
from refact_self_hosting.webgui import tab_finetune


app = FastAPI(docs_url=None, redoc_url=None)
app.include_router(selfhost_fastapi_completions.router, prefix="/v1")
app.include_router(selfhost_fastapi_gpu.router, prefix="/infengine-v1")
app.include_router(tab_upload.router)
app.include_router(tab_finetune.router)
app.include_router(selfhost_static.router)


origins = [
]


app.add_middleware(
    CORSMiddleware,
    allow_origins=origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


if __name__ == "__main__":
    class MyLogHandler(logging.Handler):
        def emit(self, record):
            timestamp = datetime.datetime.now().strftime("%Y%m%d %H:%M:%S")
            sys.stderr.write(timestamp + " " + self.format(record) + "\n")
            sys.stderr.flush()
    handler = MyLogHandler()
    handler.setLevel(logging.INFO)
    handler.setFormatter(logging.Formatter('WEBUI %(message)s'))
    root = logging.getLogger()
    root.addHandler(handler)
    root.setLevel(logging.INFO)

    def handle_sigint(*args):
        print("Received SIGINT, exiting...")
        exit(1)

    @app.on_event("startup")
    async def startup_event():
        signal.signal(signal.SIGINT, handle_sigint)

    uvicorn.run(app,
        workers=1,
        host="127.0.0.1",
        port=8008,
        log_config=None,
        # debug=True,
        # loop="asyncio",
    )
