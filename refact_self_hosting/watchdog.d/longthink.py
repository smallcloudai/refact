from refact_self_hosting import env
import os
import json

config_file = os.path.join(env.DIR_CONFIG, "openai.json")


def can_start():
    if not os.path.exists(config_file):
        return False
    with open(config_file, 'rb') as f:
        config = json.load(f)

    if config.get('enabled', False) and len(config.get('api_key', "")) != 0:
        os.environ["OPENAI_API_KEY"] = config.get('api_key')
        return True

    return False


def need_shutdown():
    if not os.path.exists(config_file):
        return True
    with open(config_file, 'rb') as f:
        config = json.load(f)

    return not config.get('enabled', False)
