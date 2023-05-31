import asyncio
import uvloop
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
    import uvicorn
    uvicorn.run(app,
        workers=1,
        host="127.0.0.1",
        port=8008,
        # debug=True,
        # loop="asyncio",
    )
