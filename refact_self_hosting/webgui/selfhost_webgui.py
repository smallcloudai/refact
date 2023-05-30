import asyncio
import uvloop
asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
# from deploy_front_py import fastapi_nlp
# from deploy_front_py import fastapi_infengine
from refact_self_hosting.webgui import selfhost_static


app = FastAPI(docs_url=None, redoc_url=None)
# app.include_router(fastapi_nlp.router, prefix="/v1")
# app.include_router(fastapi_infengine.router, prefix="/infengine-v1")
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
