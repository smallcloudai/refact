import json
import os

from pydantic import BaseModel, Field
from typing import Dict, List

from refact_utils.scripts import env
from refact_webgui.webgui.selfhost_webutils import log


class ThirdPartyProviderConfig(BaseModel):
    provider_name: str
    api_key: str
    enabled: bool
    enabled_models: List[str] = Field(default_factory=list)


class ThirdPartyApiConfig(BaseModel):
    providers: Dict[str, ThirdPartyProviderConfig] = Field(default_factory=dict)


# TODO: migration logic
def _migrate_third_party_config():
    """
    Migrate from the old configuration format to the new one.
    """
    try:
        # Load the old API keys
        api_keys = {}
        if os.path.exists(env.CONFIG_INTEGRATIONS):
            with open(str(env.CONFIG_INTEGRATIONS), "r") as f:
                api_keys = json.load(f)

        # Load the old enabled models
        enabled_models = {}
        if os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
            with open(str(env.CONFIG_INTEGRATIONS_MODELS), "r") as f:
                enabled_models = json.load(f)

        # Create the new configuration
        providers_dict = {}
        for provider_id, api_key in api_keys.items():
            providers_dict[provider_id] = ThirdPartyProviderConfig(
                provider_name=provider_id,
                api_key=api_key,
                enabled=True,
                enabled_models=enabled_models.get(provider_id, []),
            )

        # Save the new configuration
        config = ThirdPartyApiConfig(providers=providers_dict)
        save_third_party_config(config)

        # Rename the old configuration file to .bak
        if os.path.exists(env.CONFIG_INTEGRATIONS):
            os.rename(env.CONFIG_INTEGRATIONS, env.CONFIG_INTEGRATIONS + ".bak")
    except Exception as e:
        # If migration fails, log the error and continue
        log(f"Error migrating old configuration: {e}")


def load_third_party_config() -> ThirdPartyApiConfig:
    """
    Load the third-party API configuration from the file.
    If the file doesn't exist or is invalid, return an empty configuration.
    """
    # Check if the old config exists and migrate it
    # if os.path.exists(env.CONFIG_INTEGRATIONS) and not os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
    #     _migrate_third_party_config()

    try:
        if not os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
            raise FileNotFoundError(f"No third party config found")
        with open(env.CONFIG_INTEGRATIONS_MODELS, "r") as f:
            data = json.load(f)
        return ThirdPartyApiConfig.model_validate(data)
    except Exception as e:
        log(f"Can't read third-party providers config, fallback to empty: {e}")
        return ThirdPartyApiConfig()


def save_third_party_config(config: ThirdPartyApiConfig):
    """
    Save the third-party API configuration to the file.
    """
    # Create the directory if it doesn't exist
    os.makedirs(os.path.dirname(env.CONFIG_INTEGRATIONS_MODELS), exist_ok=True)

    # Save the configuration
    with open(env.CONFIG_INTEGRATIONS_MODELS + ".tmp", "w") as f:
        json.dump(config.model_dump(), f, indent=4)
    os.rename(env.CONFIG_INTEGRATIONS_MODELS + ".tmp", env.CONFIG_INTEGRATIONS_MODELS)
