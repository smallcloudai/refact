import os
import uuid

from fastapi import APIRouter
from fastapi import Query
from fastapi.exceptions import HTTPException
from fastapi.responses import FileResponse
from fastapi.responses import JSONResponse

from pydantic import BaseModel

from refact_utils.scripts import env
from refact_webgui.webgui import static_folders

from typing import List


__all__ = ["RefactSession", "DummySession", "AdminSession", "AdminRouter"]


class Credentials(BaseModel):
    token: str


class RefactSession:

    @property
    def exclude_routes(self) -> List[str]:
        raise NotImplementedError()

    def authorize(self, token: str) -> str:
        raise NotImplementedError()

    def authenticate(self, session_key: str) -> bool:
        raise NotImplementedError()

    def header_authenticate(self, authorization: str) -> str:
        raise NotImplementedError()


class DummySession(RefactSession):

    @property
    def exclude_routes(self) -> List[str]:
        return []

    def authorize(self, token: str) -> str:
        return ""

    def authenticate(self, session_key: str) -> bool:
        return True

    def header_authenticate(self, authorization: str) -> str:
        return "user"


class AdminSession(RefactSession):

    def __init__(self, token: str):
        self._token = token
        if os.path.exists(env.ADMIN_SESSION_KEY):
            with open(env.ADMIN_SESSION_KEY, "r") as f:
                self._session_key = f.read()
        else:
            self._session_key = self._generate_session_key()

    @property
    def exclude_routes(self) -> List[str]:
        return [
            "/admin",
            "/coding_assistant_caps.json",
            "/refact-caps",
            "/tokenizer",
            "/customization",
            "/v1",
            "/infengine-v1",
            "/stats/telemetry",
            "/stats/rh-stats",
            "/chat",
            "/assets",  # TODO: this static dir should be renamed soon
            "/favicon.png",
            "/lsp",
        ]

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

    def header_authenticate(self, authorization: str) -> str:
        if authorization is None:
            raise ValueError("Missing authorization header")
        bearer_hdr = authorization.split(" ")
        if len(bearer_hdr) != 2 or bearer_hdr[0] != "Bearer":
            raise ValueError("Invalid authorization header")
        api_key = bearer_hdr[1]
        if self._token == api_key:
            return "user"
        raise ValueError("API key mismatch")


class AdminRouter(APIRouter):

    def __init__(self, session: RefactSession, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._session = session
        self.add_api_route("", self._get_login_page, methods=["GET"])
        self.add_api_route("", self._login, methods=["POST"])

    async def _get_login_page(self):
        for spath in static_folders:
            fn = os.path.join(spath, "admin.html")
            if os.path.exists(fn):
                return FileResponse(fn, media_type="text/html")
        raise HTTPException(404, "No admin.html found")

    async def _login(self, credentials: Credentials):
        try:
            self._session_key = self._session.authorize(token=credentials.token)
            return JSONResponse(status_code=200, content={"session_key": self._session_key})
        except ValueError:
            raise HTTPException(status_code=401, detail="Invalid credentials")
