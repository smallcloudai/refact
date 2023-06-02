import os

PERMDIR = os.environ.get("SMC_PERM_DIR", "") or os.path.expanduser("~/.smc/perm-storage")
TMPDIR = os.environ.get("SMC_TMP_DIR", "") or os.path.expanduser("~/.smc/tmp")

DIR_CONFIG  = os.path.join(PERMDIR, "cfg")
DIR_WEIGHTS = os.path.join(PERMDIR, "weights")
DIR_LORAS   = os.path.join(PERMDIR, "loras")
DIR_LOGS    = os.path.join(PERMDIR, "logs")
DIR_UPLOADS = os.path.join(PERMDIR, "uploaded-files")

DIR_UNPACKED = os.path.join(TMPDIR, "unpacked-files")

CONFIG_FINETUNE = os.path.join(DIR_CONFIG, "tab_finetune.cfg")
CONFIG_HOW_TO_PROCESS = os.path.join(DIR_CONFIG, "how_to_process.cfg")
CONFIG_PROCESSING_STATS = os.path.join(DIR_CONFIG, "processing_stats.cfg")

FLAG_LAUNCH_PROCESS_UPLOADS = os.path.join(DIR_CONFIG, "_launch_process_uploaded.flag")
FLAG_LAUNCH_FINETUNE = os.path.join(DIR_CONFIG, "_launch_finetune.flag")

os.makedirs(DIR_CONFIG, exist_ok=True)
os.makedirs(DIR_WEIGHTS, exist_ok=True)
os.makedirs(DIR_LORAS, exist_ok=True)
os.makedirs(DIR_LOGS, exist_ok=True)
os.makedirs(DIR_UPLOADS, exist_ok=True)

os.makedirs(DIR_UNPACKED, exist_ok=True)
