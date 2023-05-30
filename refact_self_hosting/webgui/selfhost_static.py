import os
from fastapi import APIRouter
from fastapi.responses import FileResponse


router = APIRouter()

this_file_dir = os.path.dirname(os.path.abspath(__file__))


@router.get("/")
async def index():
    html_path = os.path.join(this_file_dir, "static", "index.html")
    return FileResponse(html_path, media_type="text/html")


@router.get("/{file_path:path}")
async def static_file(file_path: str):
    if ".." in file_path:
        raise Exception("Invalid file path")
    static_path = os.path.join(this_file_dir, "static", file_path)
    return FileResponse(static_path)


@router.get("/ping")
def ping_handler():
    return {"message": "pong"}
