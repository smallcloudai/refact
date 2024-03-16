import os
import logging

PERMDIR = os.environ.get("REFACT_PERM_DIR", "") or os.path.expanduser("~/.refact/perm-storage")
TMPDIR = os.environ.get("REFACT_TMP_DIR", "") or os.path.expanduser("~/.refact/tmp")
FLAG_FACTORY_RESET = os.path.join(PERMDIR, "_factory_reset.flag")

DIR_CONFIG     = os.path.join(PERMDIR, "cfg")
DIR_WATCHDOG_D = os.path.join(PERMDIR, "cfg", "watchdog.d")
DIR_WEIGHTS    = os.path.join(PERMDIR, "weights")
DIR_LORAS      = os.path.join(PERMDIR, "loras")
DIR_LOGS       = os.path.join(PERMDIR, "logs")
DIR_PROJECTS   = os.path.join(PERMDIR, "projects")
DIR_SSH_KEYS   = os.path.join(PERMDIR, "ssh-keys")

CONFIG_INTEGRATIONS = os.path.join(DIR_CONFIG, "integrations.cfg")
CONFIG_ENUM_GPUS = os.path.join(DIR_CONFIG, "gpus_enum_result.out")
CONFIG_BUSY_GPUS = os.path.join(DIR_CONFIG, "gpus_busy_result.out")
CONFIG_INFERENCE = os.path.join(DIR_CONFIG, "inference.cfg")
CONFIG_ACTIVE_LORA = os.path.join(DIR_CONFIG, "inference_active_lora.cfg")

# Per project:
PP_DIR_UPLOADS             = lambda pname: os.path.join(PERMDIR, "projects", pname, "uploaded-files")
PP_CONFIG_HOW_TO_UNZIP     = lambda pname: os.path.join(PERMDIR, "projects", pname, "sources_scan.cfg")
PP_CONFIG_HOW_TO_FILETYPES = lambda pname: os.path.join(PERMDIR, "projects", pname, "sources_filetypes.cfg")
PP_CONFIG_PROCESSING_STATS = lambda pname: os.path.join(PERMDIR, "projects", pname, "sources_stats.out")
PP_LOG_FILES_ACCEPTED_SCAN = lambda pname: os.path.join(PERMDIR, "projects", pname, "files_accepted_scan.log")
PP_LOG_FILES_REJECTED_SCAN = lambda pname: os.path.join(PERMDIR, "projects", pname, "files_rejected_scan.log")
PP_LOG_FILES_ACCEPTED_FTF  = lambda pname: os.path.join(PERMDIR, "projects", pname, "files_accepted_ftf.log")
PP_LOG_FILES_REJECTED_FTF  = lambda pname: os.path.join(PERMDIR, "projects", pname, "files_rejected_ftf.log")
PP_SCAN_STATUS             = lambda pname: os.path.join(PERMDIR, "projects", pname, "scan_status.out")
PP_CONFIG_FINETUNE_FILTER_STAT = lambda pname: os.path.join(PERMDIR, "projects", pname, "finetune_filter_stats.out")

PP_DIR_UNPACKED = lambda pname: os.path.join(PERMDIR, "projects", pname, "unpacked")
PP_TRAIN_UNFILTERED_FILEPATH = lambda pname: os.path.join(PP_DIR_UNPACKED(pname), "train_set.jsonl")
PP_TRAIN_FILTERED_FILEPATH   = lambda pname: os.path.join(PP_DIR_UNPACKED(pname), "train_set_filtered.jsonl")
PP_TEST_UNFILTERED_FILEPATH  = lambda pname: os.path.join(PP_DIR_UNPACKED(pname), "test_set.jsonl")
PP_TEST_FILTERED_FILEPATH    = lambda pname: os.path.join(PP_DIR_UNPACKED(pname), "test_set_filtered.jsonl")
PP_LOSS_PER_HASH_DB_FILEPATH = lambda pname: os.path.join(PP_DIR_UNPACKED(pname), "loss_per_hash_db.json")
PP_PROJECT_LOCK              = lambda pname: os.path.join(PP_DIR_UNPACKED(pname), "project.lock")


# finetune
CONFIG_FINETUNE = os.path.join(DIR_CONFIG, "finetune.cfg")    # non project-specific config to start again
CONFIG_HOW_TO_FILTER = os.path.join(DIR_CONFIG, "finetune_filter.cfg")

ADMIN_SESSION_KEY = os.path.join(DIR_CONFIG, "admin_session.key")

# FLAG_LAUNCH_PROCESS_UPLOADS = os.path.join(DIR_WATCHDOG_D, "_launch_process_uploaded.flag")
# FLAG_LAUNCH_FINETUNE_FILTER_ONLY = os.path.join(DIR_WATCHDOG_D, "_launch_finetune_filter_only.flag")
# FLAG_LAUNCH_FINETUNE = os.path.join(DIR_WATCHDOG_D, "ftune.flag")
# FLAG_STOP_FILTER = os.path.join(DIR_WATCHDOG_D, "_stop_filter.flag")

def create_dirs():
    os.makedirs(DIR_WATCHDOG_D, exist_ok=True)
    os.makedirs(DIR_WEIGHTS, exist_ok=True)
    os.makedirs(DIR_LORAS, exist_ok=True)
    os.makedirs(DIR_LOGS, exist_ok=True)
    os.makedirs(DIR_SSH_KEYS, exist_ok=True)

create_dirs()

# env mechanism doesn't work for this variable well
DIR_WATCHDOG_TEMPLATES = os.path.join(
    os.path.dirname(__file__), "..", "..", "self_hosting_machinery", "watchdog", "watchdog.d")


private_key_ext = 'private_key'
fingerprint_ext = 'fingerprint'


def get_all_ssh_keys():
    import glob
    return glob.glob(f'{DIR_SSH_KEYS}/*.{private_key_ext}')

def report_status(program, status):
    assert program in ["linguist", "filter", "ftune"]
    assert status in ["starting", "working", "finished", "failed", "interrupted"]
    if status == "starting":  # reported only by watchdog
        return
    if status == "finished":  # reported only by watchdog
        return
    if status == "failed":  # reported only by watchdog
        return
    if status == "interrupted":  # reported only by watchdog
        return
    logging.info("STATUS %s" % (status))
