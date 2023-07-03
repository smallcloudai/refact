import json
import os

from fastapi import APIRouter
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from pathlib import Path

from refact_self_hosting.env import DIR_SSH_KEYS, private_key_ext, fingerprint_ext, get_all_ssh_keys, \
    DIR_WATCHDOG_TEMPLATES, CHATGPT_CONFIG_FILENAME, DIR_WATCHDOG_D

__all__ = ["TabSettingsRouter"]


class TabSettingsRouter(APIRouter):
    __template_longthink_cfg = os.path.join(DIR_WATCHDOG_TEMPLATES, "longthink.cfg")
    __longthink_cfg = os.path.join(DIR_WATCHDOG_D, "longthink.cfg")

    __default_chatgpt_config = dict(is_enabled=False, api_key="")
    class SSHKey(BaseModel):
        name: str
    class ChatGPTIsEnabled(BaseModel):
        is_enabled: bool
    class ChatGPTApiKey(BaseModel):
        api_key: str

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-settings-create-ssh-key", self._tab_settings_create_ssh_key, methods=["POST"])
        self.add_api_route("/tab-settings-delete-ssh-key", self._tab_settings_delete_ssh_key, methods=["POST"])
        self.add_api_route("/tab-settings-get-all-ssh-keys", self._tab_settings_get_all_ssh_keys, methods=["GET"])
        self.add_api_route("/tab-api-key-settings-set-enabled-chat-gpt",
                           self._tab_api_key_settings_set_enabled_chat_gpt, methods=["POST"])
        self.add_api_route("/tab-api-key-settings-set-chat-gpt-api-key",
                           self._tab_api_key_settings_set_chat_gpt_api_key, methods=["POST"])
        self.add_api_route("/tab-api-key-settings-get-chat-gpt-info",
                           self._tab_api_key_settings_get_chat_gpt_info, methods=["GET"])
        self.__init_longthink_process()

    def __init_longthink_process(self):
        if os.path.exists(CHATGPT_CONFIG_FILENAME):
            with open(CHATGPT_CONFIG_FILENAME, 'r') as f:
                openai_config = json.load(f)
        else:
            openai_config = self.__default_chatgpt_config

        self.__enable_longthink_process(openai_config)

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

            with open(f'{DIR_SSH_KEYS}/{data.name}.{private_key_ext}', 'wb') as f:
                f.write(private_key)

            public_key = key.public_key().public_bytes(
                crypto_serialization.Encoding.OpenSSH,
                crypto_serialization.PublicFormat.OpenSSH
            )

            digest = hashlib.sha256(binascii.a2b_base64(public_key[8:])).digest()
            fingerprint = "SHA256:" + base64.b64encode(digest).rstrip(b'=').decode('utf-8')
            with open(f'{DIR_SSH_KEYS}/{data.name}.{fingerprint_ext}', 'w') as f:
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
            fingerprint_file = key_file.with_suffix(f'.{fingerprint_ext}')
            with open(str(fingerprint_file), 'r') as f:
                fingerprint = f.read()
            return dict(
                name=key_file.stem,
                created_ts=key_file.stat().st_mtime,
                fingerprint=fingerprint
            )

        return JSONResponse([
            get_info_from_key(ssh_key) for ssh_key in get_all_ssh_keys()
        ])

    async def _tab_settings_delete_ssh_key(self, data: SSHKey):
        key_filepath = Path(DIR_SSH_KEYS) / f'{data.name}.{private_key_ext}'
        fingerprint_filepath = Path(DIR_SSH_KEYS) / f'{data.name}.{fingerprint_ext}'
        if key_filepath.exists():
            key_filepath.unlink(missing_ok=False)
        if fingerprint_filepath.exists():
            fingerprint_filepath.unlink(missing_ok=False)
        return JSONResponse("OK")

    def __inject_in_openai_config(self, upd: dict):
        if os.path.exists(CHATGPT_CONFIG_FILENAME):
            with open(str(CHATGPT_CONFIG_FILENAME), "r") as f:
                config = json.load(f)
        else:
            config = self.__default_chatgpt_config
        config.update(upd)
        tmp = f'{CHATGPT_CONFIG_FILENAME}.tmp'
        with open(str(tmp), "w") as f:
            json.dump(config, f)
        os.rename(tmp, CHATGPT_CONFIG_FILENAME)
        return config

    def __enable_longthink_process(self, openai_config: dict):
        if not openai_config.get('is_enabled', False):
            if os.path.exists(self.__longthink_cfg):
                os.remove(self.__longthink_cfg)
            return
        with open(self.__template_longthink_cfg, 'r') as f:
            config = json.load(f)
        config.pop('unfinished')
        config['command_line'].append('--openai_key')
        config['command_line'].append(openai_config.get('api_key', "dummy"))
        tmp = f'{self.__longthink_cfg}.tmp'
        with open(tmp, 'w') as f:
            json.dump(config, f)
        os.rename(tmp, self.__longthink_cfg)

    async def _tab_api_key_settings_set_enabled_chat_gpt(self, data: ChatGPTIsEnabled):
        config = self.__inject_in_openai_config(dict(is_enabled=data.is_enabled))
        self.__enable_longthink_process(config)
        return JSONResponse("OK")

    async def _tab_api_key_settings_set_chat_gpt_api_key(self, data: ChatGPTApiKey):
        config = self.__inject_in_openai_config(dict(api_key=data.api_key))
        self.__enable_longthink_process(config)
        return JSONResponse("OK")

    async def _tab_api_key_settings_get_chat_gpt_info(self):
        if os.path.exists(CHATGPT_CONFIG_FILENAME):
            with open(str(CHATGPT_CONFIG_FILENAME), "r") as f:
                config = json.load(f)
        else:
            config = self.__default_chatgpt_config
        return JSONResponse(config)

