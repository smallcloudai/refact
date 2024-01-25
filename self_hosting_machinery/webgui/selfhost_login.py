import os
import uuid

from fastapi import APIRouter
from fastapi import Query
from fastapi.exceptions import HTTPException
from fastapi.responses import FileResponse
from fastapi.responses import JSONResponse

from pydantic import BaseModel
from pydantic import Required

from self_hosting_machinery.webgui import static_folders
from self_hosting_machinery import env


__all__ = ["LoginRouter"]


class Credentials(BaseModel):
    token: str = Query(default=Required)


class AdminSession:

    def __init__(self):
        self._token = os.environ.get("REFACT_ADMIN_TOKEN", "12345")
        if os.path.exists(env.ADMIN_SESSION_KEY):
            with open(env.ADMIN_SESSION_KEY, "r") as f:
                self._session_key = f.read()
        else:
            self._session_key = self._generate_session_key()

    @staticmethod
    def _generate_session_key() -> str:
        return str(uuid.uuid4())

    def _set_session_key(self, session_key: str):
        with open(env.ADMIN_SESSION_KEY, "w") as f:
            f.write(session_key)
        self._session_key = session_key

    def authorize(self, token: str) -> str:
        if self._token == token:
            session_key = self._generate_session_key()
            self._set_session_key(session_key)
            return session_key
        raise ValueError("Invalid token")

    def authenticate(self, session_key: str) -> bool:
        if not isinstance(session_key, str):
            return False
        return session_key == self._session_key


class LoginRouter(APIRouter):

    def __init__(self, session: AdminSession, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._session = session
        self.add_api_route("", self._get_login_page, methods=["GET"])
        self.add_api_route("", self._login, methods=["POST"])

    async def _get_login_page(self):
        for spath in static_folders:
            fn = os.path.join(spath, "login.html")
            if os.path.exists(fn):
                return FileResponse(fn, media_type="text/html")
        raise HTTPException(404, "No login.html found")

    async def _login(self, credentials: Credentials):
        try:
            self._session_key = self._session.authorize(token=credentials.token)
            return JSONResponse(status_code=200, content={"session_key": self._session_key})
        except ValueError:
            raise HTTPException(status_code=401, detail="Invalid credentials")
