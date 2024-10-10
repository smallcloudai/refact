import os
import yaml
from pydantic import BaseModel, ValidationError
from typing import Optional, Dict, List
from prompt_toolkit.enums import EditingMode
import aiohttp


class CapsModel(BaseModel):
    n_ctx: int
    similar_models: List[str]
    supports_tools: bool


class Caps(BaseModel):
    cloud_name: str
    code_chat_models: Dict[str, CapsModel]
    code_chat_default_model: str
    embedding_model: str


class SettingsCLI(BaseModel):
    address_url: str
    api_key: str
    insecure_ssl: bool = False
    ast: bool = True
    ast_max_files: int = 20000
    vecdb: bool = True
    vecdb_max_files: int = 5000
    experimental: bool = False
    basic_telemetry: bool = True
    nerd_font: bool = False
    editing_mode: str = "default"

    def get_editing_mode(self):
        if self.editing_mode.lower() in ["vim", "vi"]:
            return EditingMode.VI
        else:
            return EditingMode.EMACS


default_config = """
# The caps file is bring-your-own-key.yaml by default, that in turn works with OPENAI_API_KEY inside by default.
# But you can change it to:
#address_url: Refact
#api_key: <take-from-website>
#address_url: http://your-self-hosting-server/
#api_key: your-secret-key

# Accept self-signed certificates
#insecure_ssl: true

ast: true
ast_max_files: 20000
vecdb: true
vecdb_max_files: 5000

#experimental: true
#basic_telemetry: false
#nerd_font: true
#editing_mode: vim
#editing_mode: emacs
"""


class CmdlineSettings:
    def __init__(self, caps: Caps, args):
        self.caps = caps
        self.model = args.model or caps.code_chat_default_model
        self.project_path = args.path_to_project

    def n_ctx(self):
        return self.caps.code_chat_models[self.model].n_ctx


args: Optional[CmdlineSettings] = None
cli_yaml: Optional[SettingsCLI] = None


async def fetch_caps(base_url: str) -> Caps:
    url = f"{base_url}/caps"
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            if response.status == 200:
                data = await response.json()
                return Caps(**data)  # Parse the JSON data into the Caps model
            else:
                raise RuntimeError(f"cannot fetch {url}\n{response.status}")


def load_cli_or_auto_configure():
    cli_yaml_path = os.path.expanduser("~/.cache/refact/cli.yaml")
    if not os.path.exists(cli_yaml_path):
        # No config, autodetect
        print("First run. Welcome, I'll try to set up a reasonable config.")
        from pathlib import Path
        Path(cli_yaml_path).parent.mkdir(parents=True, exist_ok=True)
        with open(cli_yaml_path, 'w') as file:
            file.write(default_config)
    with open(cli_yaml_path, 'r') as file:
        data = yaml.safe_load(file)
        try:
            return SettingsCLI.model_validate(data)
        except ValidationError as exc:
            print(f'Warning: Invalid configuration found in {cli_yaml_path}.')
            print('The following errors were detected:')
            for err in exc.errors():
                print(f'  - {err["type"]}: {err["loc"][0]}')
            exit()
