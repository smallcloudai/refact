import os

PERMDIR = os.environ.get("REFACT_PERM_DIR", "") or os.path.expanduser("~/.refact/perm-storage")
TMPDIR = os.environ.get("REFACT_TMP_DIR", "") or os.path.expanduser("~/.refact/tmp")

DIR_CONFIG     = os.path.join(PERMDIR, "cfg")
DIR_WATCHDOG_D = os.path.join(PERMDIR, "cfg", "watchdog.d")
DIR_WEIGHTS    = os.path.join(PERMDIR, "weights")
DIR_LORAS      = os.path.join(PERMDIR, "loras")
DIR_LOGS       = os.path.join(PERMDIR, "logs")
DIR_UPLOADS    = os.path.join(PERMDIR, "uploaded-files")
DIR_SSH_KEYS   = os.path.join(PERMDIR, "ssh-keys")

DIR_UNPACKED = os.path.join(TMPDIR, "unpacked-files")

CONFIG_ENUM_GPUS = os.path.join(DIR_CONFIG, "enum_gpus_result.cfg")
CONFIG_INFERENCE = os.path.join(DIR_CONFIG, "inference.cfg")
CONFIG_ACTIVE_LORA = os.path.join(DIR_CONFIG, "inference_active_lora.cfg")
CONFIG_HOW_TO_UNZIP = os.path.join(DIR_CONFIG, "sources_unzip.cfg")
CONFIG_HOW_TO_FILETYPES = os.path.join(DIR_CONFIG, "sources_filetypes.cfg")
CONFIG_PROCESSING_STATS = os.path.join(DIR_CONFIG, "sources_stats.cfg")
CONFIG_FINETUNE = os.path.join(DIR_CONFIG, "finetune.cfg")
CONFIG_HOW_TO_FILTER = os.path.join(DIR_CONFIG, "finetune_smart_filter.cfg")

FLAG_LAUNCH_PROCESS_UPLOADS = os.path.join(DIR_WATCHDOG_D, "_launch_process_uploaded.flag")
FLAG_LAUNCH_FINETUNE = os.path.join(DIR_WATCHDOG_D, "_launch_finetune.flag")

os.makedirs(DIR_WATCHDOG_D, exist_ok=True)
os.makedirs(DIR_WEIGHTS, exist_ok=True)
os.makedirs(DIR_LORAS, exist_ok=True)
os.makedirs(DIR_LOGS, exist_ok=True)
os.makedirs(DIR_UPLOADS, exist_ok=True)
os.makedirs(DIR_SSH_KEYS, exist_ok=True)

os.makedirs(DIR_UNPACKED, exist_ok=True)

DIR_WATCHDOG_TEMPLATES = os.path.join(os.path.dirname(__file__), "watchdog.d")

GIT_CONFIG_FILENAME = 'git_config.json'
CHATGPT_CONFIG_FILENAME = os.path.join(DIR_CONFIG, "openai.cfg")

private_key_ext = 'private_key'
fingerprint_ext = 'fingerprint'


def get_all_ssh_keys():
    import glob
    return glob.glob(f'{DIR_SSH_KEYS}/*.{private_key_ext}')
