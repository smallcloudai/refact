import logging
import asyncio
import uvloop
import sys
import datetime
import signal
import uvicorn
import weakref

asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from refact_self_hosting.webgui.selfhost_req_queue import Ticket
from refact_self_hosting.webgui.selfhost_static import StaticRouter
from refact_self_hosting.webgui.selfhost_fastapi_completions import CompletionsRouter
from refact_self_hosting.webgui.selfhost_fastapi_gpu import GPURouter
from refact_self_hosting.webgui.tab_upload import TabUploadRouter
from refact_self_hosting.webgui.tab_finetune import TabFinetuneRouter

from collections import defaultdict
from typing import Dict


user2gpu_queue: Dict[str, asyncio.Queue] = defaultdict(asyncio.Queue)   # for each model there is a queue
id2ticket: Dict[str, Ticket] = weakref.WeakValueDictionary()


app = FastAPI(docs_url=None, redoc_url=None)
app.include_router(StaticRouter())
app.include_router(CompletionsRouter(prefix="/v1", id2ticket=id2ticket, user2gpu_queue=user2gpu_queue))
app.include_router(GPURouter(prefix="/infengine-v1", id2ticket=id2ticket, user2gpu_queue=user2gpu_queue))
app.include_router(TabUploadRouter())
app.include_router(TabFinetuneRouter())


app.add_middleware(
    CORSMiddleware,
    allow_origins=[],
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
