import json
import os

from fastapi import APIRouter
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from pathlib import Path
from self_hosting_machinery import env  # REFACTORME
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner

from typing import Optional


__all__ = ["TabSettingsRouter"]


class TabSettingsRouter(APIRouter):
    class SSHKey(BaseModel):
        name: str

    class Integrations(BaseModel):
        openai_api_key: Optional[str] = None
        anthropic_api_key: Optional[str] = None

    def __init__(self, models_assigner: ModelAssigner, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._models_assigner = models_assigner
        self.add_api_route("/tab-settings-integrations-get", self._tab_settings_integrations_get, methods=["GET"])
        self.add_api_route("/tab-settings-integrations-save", self._tab_settings_integrations_save, methods=["POST"])
        self.add_api_route("/tab-settings-create-ssh-key", self._tab_settings_create_ssh_key, methods=["POST"])
        self.add_api_route("/tab-settings-delete-ssh-key", self._tab_settings_delete_ssh_key, methods=["POST"])
        self.add_api_route("/tab-settings-get-all-ssh-keys", self._tab_settings_get_all_ssh_keys, methods=["GET"])
        self.add_api_route("/tab-settings-factory-reset", self._tab_settings_factory_reset, methods=["GET"])

    async def _tab_settings_integrations_get(self):
        if os.path.exists(env.CONFIG_INTEGRATIONS):
            with open(str(env.CONFIG_INTEGRATIONS), "r") as f:
                config = json.load(f)
        else:
            config = {}
        return JSONResponse(config)

    async def _tab_settings_integrations_save(self, data: Integrations):
        with open(env.CONFIG_INTEGRATIONS + ".tmp", "w") as f:
            json.dump({
                k: v
                for k, v in data.dict().items()
                if v is not None
            }, f, indent=4)
        os.rename(env.CONFIG_INTEGRATIONS + ".tmp", env.CONFIG_INTEGRATIONS)
        self._models_assigner.models_to_watchdog_configs()
        return JSONResponse("OK")

    async def _tab_settings_create_ssh_key(self, data: SSHKey):
        try:
            from cryptography.hazmat.primitives import serialization as crypto_serialization
            from cryptography.hazmat.primitives.asymmetric import rsa
            from cryptography.hazmat.backends import default_backend as crypto_default_backend
            import binascii
            import base64
            import hashlib

            key = rsa.generate_private_key(
                backend=crypto_default_backend(),
                public_exponent=65537,
                key_size=4096
            )

            private_key = key.private_bytes(
                encoding=crypto_serialization.Encoding.PEM,
                format=crypto_serialization.PrivateFormat.TraditionalOpenSSL,
                encryption_algorithm=crypto_serialization.NoEncryption(),
            )

            with open(f'{env.DIR_SSH_KEYS}/{data.name}.{env.private_key_ext}', 'wb') as f:
                f.write(private_key)

            os.chmod(f'{env.DIR_SSH_KEYS}/{data.name}.{env.private_key_ext}', 0o400, follow_symlinks=True)

            public_key = key.public_key().public_bytes(
                crypto_serialization.Encoding.OpenSSH,
                crypto_serialization.PublicFormat.OpenSSH
            )

            digest = hashlib.sha256(binascii.a2b_base64(public_key[8:])).digest()
            fingerprint = "SHA256:" + base64.b64encode(digest).rstrip(b'=').decode('utf-8')
            with open(f'{env.DIR_SSH_KEYS}/{data.name}.{env.fingerprint_ext}', 'w') as f:
                f.write(fingerprint)

            return JSONResponse({
                "name": data.name,
                "public_key": public_key.decode("utf-8"),
                "fingerprint": fingerprint
            })
        except Exception as e:
            response_data = {"message": f"Error: {e}"}
            return JSONResponse(response_data, status_code=500)

    async def _tab_settings_get_all_ssh_keys(self):
        def get_info_from_key(key_path: str):
            key_file = Path(key_path)
            fingerprint_file = key_file.with_suffix(f'.{env.fingerprint_ext}')
            with open(str(fingerprint_file), 'r') as f:
                fingerprint = f.read()
            return dict(
                name=key_file.stem,
                created_ts=key_file.stat().st_mtime,
                fingerprint=fingerprint
            )

        return JSONResponse([
            get_info_from_key(ssh_key) for ssh_key in env.get_all_ssh_keys()
        ])

    async def _tab_settings_delete_ssh_key(self, data: SSHKey):
        key_filepath = Path(env.DIR_SSH_KEYS) / f'{data.name}.{env.private_key_ext}'
        fingerprint_filepath = Path(env.DIR_SSH_KEYS) / f'{data.name}.{env.fingerprint_ext}'
        if key_filepath.exists():
            key_filepath.unlink(missing_ok=False)
        if fingerprint_filepath.exists():
            fingerprint_filepath.unlink(missing_ok=False)
        return JSONResponse("OK")

    async def _tab_settings_factory_reset(self):
        with open(env.FLAG_FACTORY_RESET, "w") as f:
            pass
