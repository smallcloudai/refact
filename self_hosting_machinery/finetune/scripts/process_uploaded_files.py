import click
import subprocess
import multiprocessing
import os
import json
import sys
import time
import jsonlines
import logging
import filelock

from itertools import chain
from collections import Counter
from fnmatch import fnmatch

from refact_utils.scripts import env
import self_hosting_machinery.finetune.utils.traces as traces

from typing import List, Dict, Any, Iterable, Tuple


EXE = os.path.expanduser("~/code/linguist/bin/smc-linguist")
if not os.path.exists(EXE):
    EXE = "smc-linguist"  # Rely on PATH
CWD = os.getcwd()

GIT_EXE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../../refact_data_pipeline/git_command.exp')

TRUSTED_LANGUAGES = {
    'Assembly', 'Batchfile', 'C', 'C#', 'C++', 'CMake', 'CSS', 'Cuda', 'Dockerfile', 'Fortran',
    'Go', 'HTML', 'Haskell', 'Java', 'JavaScript', 'Kotlin', 'Lua', 'M', 'Makefile', 'Markdown',
    'PHP', 'Perl', 'Python', 'R', 'Ruby', 'Rust', 'SQL', 'Scala', 'Shell', 'TeX', 'TypeScript',
}

stats_json = {
    "scan_finished": False,
    "scan_finished_ts": "",
    "files_before_dedup": 0,
    "files_after_dedup": 0,
    "filestats_scan_finetune": {
        "accepted": 0,
        "rejected": 0,
    },
    "filestats_scan_db": {
        "accepted": 0,
        "rejected": 0,
    },
    "uploaded_files": {},
}


def log(*args):
    s = " ".join(map(str, args))
    if traces.context():
        traces.log(s)
    else:
        logging.info(s)


def stats_save(pname):
    traces.touch()
    env.report_status("linguist", stats_json["scan_status"])
    with open(env.PP_CONFIG_PROCESSING_STATS(pname) + ".tmp", "w") as f:
        f.write(json.dumps(stats_json, indent=4))
    os.rename(env.PP_CONFIG_PROCESSING_STATS(pname) + ".tmp", env.PP_CONFIG_PROCESSING_STATS(pname))


class LinguistProcess:

    def __init__(self):
        self._process = subprocess.Popen([EXE], stdout=subprocess.PIPE, stdin=subprocess.PIPE)
        self._inside_pipe = 0

    def push(self, filename: str):
        self._process.stdin.write((filename + "\n").encode("utf-8"))
        self._process.stdin.flush()
        self._inside_pipe += 1

    def read(self) -> Dict[str, Any]:
        assert self._inside_pipe > 0
        data = self._process.stdout.readline().decode("utf-8")
        self._inside_pipe -= 1
        return json.loads(data)

    @property
    def size(self):
        return self._inside_pipe

    def close(self):
        self._process.stdin.close()
        self._process.wait()


class LinguistProcessPool:
    PIPED_MAX = 50

    def __init__(self, num_processes: int = 8):
        self._processes = [
            LinguistProcess()
            for _ in range(num_processes)
        ]

    def feed_files(self, filenames: Iterable[str]):
        for idx, filename in enumerate(filenames):
            process = self._processes[idx % len(self._processes)]
            process.push(filename)
            if process.size > self.PIPED_MAX:
                yield process.read()
        for process in self._processes:
            while process.size:
                yield process.read()
            process.close()


def ls_with_linguist(pname, start_dir):
    """
    Walk recursively through a directory, apply linguist, return dicts
    """

    def filenames_g(root_dir: str):
        for root, dirs, files in os.walk(root_dir):
            dirs[:] = [d for d in dirs if not d.startswith('.')]
            for file in files:
                p = os.path.join(root, file)
                p = os.path.abspath(p)
                assert p.startswith(env.PP_DIR_UNPACKED(pname)), "\"%s\" does not start with \"%s\"" % (p, env.PP_DIR_UNPACKED(pname))
                yield p

    num_processes = max(1, multiprocessing.cpu_count() // 2)
    linguist = LinguistProcessPool(num_processes)
    for result in linguist.feed_files(filenames_g(start_dir)):
        yield result


def get_source_type(filename: str) -> str:
    git_config = os.path.join(filename, "git_config.json")
    if os.path.isfile(filename):
        if any(filename.endswith(suffix) for suffix in [".tar.gz", ".bz2", ".zip", ".tar"]):
            return "archive"
        else:
            return "singlefile"
    elif os.path.isdir(filename) and os.path.exists(git_config) and os.path.isfile(git_config):
        return "git"
    assert 0, f"Unknown source type for {filename}"


# def source_needs_update(
#         source_type: str,
#         upload_filename: str,
#         unpack_filename: str) -> bool:
#     assert os.path.exists(upload_filename) and os.path.exists(unpack_filename)
#     if source_type in ["archive", "singlefile"]:
#         return os.path.getmtime(upload_filename) > os.path.getmtime(unpack_filename)
#     elif source_type in ["git"]:
#         completed_upload = subprocess.run(
#             ["git", "-C", upload_filename, "log", "-1", "--pretty=format:%H"],
#             stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
#         completed_unpack = subprocess.run(
#             ["git", "-C", unpack_filename, "log", "-1", "--pretty=format:%H"],
#             stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
#         return completed_unpack.stdout != completed_upload.stdout
#     else:
#         assert 0, f"Unknown source type {source_type}"


def _make_git_env():
    def _make_git_command():
        command = ['ssh', '-o', 'UserKnownHostsFile=/dev/null', '-o', 'StrictHostKeyChecking=no']
        for ssh_key in env.get_all_ssh_keys():
            command += ['-i', ssh_key]
        return ' '.join(command)

    return {
        "GIT_SSH_COMMAND": _make_git_command()
    }


def _prepare_git_repo(filepath: str, want_pull: bool) -> bool:
    sources_dir = os.path.join(filepath, "sources")

    def get_current_hash():
        hash_process = subprocess.run(
            ["git", "-C", sources_dir, "rev-parse", "HEAD"],
            env=_make_git_env(),
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL)
        if hash_process.returncode != 0:
            raise Exception(hash_process.stdout.decode())
        return hash_process.stdout.decode().splitlines()[0]

    def save_last_hash(hash: str):
        with open(os.path.join(filepath, "last_hash"), 'w') as f:
            return f.write(hash)

    def load_last_hash() -> str:
        with open(os.path.join(filepath, "last_hash"), 'r') as f:
            return f.read()

    if os.path.exists(sources_dir):
        if not want_pull:
            return False

        completed_process = subprocess.run(
            ["expect", "-f", GIT_EXE, "git", "-C", sources_dir, "pull"],
            env=_make_git_env(),
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL)
        log(f"git pull {filepath} => {'error' if completed_process.returncode else 'success'}")
        if completed_process.returncode != 0:
            raise Exception(completed_process.stdout.decode())
        last_commit_hash = load_last_hash()
        current_commit_hash = get_current_hash()
        need_to_rescan = last_commit_hash != current_commit_hash
        save_last_hash(current_commit_hash)
        return need_to_rescan

    with open(os.path.join(filepath, "git_config.json"), 'r') as f:
        config = json.load(f)

    branch_args = ["-b", config["branch"]] if config["branch"] else []
    completed_process = subprocess.run(
        ["expect", "-f", GIT_EXE, "git", "-C", filepath, "clone", "--no-recursive",
         "--depth", "1", *branch_args, config["url"], "sources"],
        env=_make_git_env(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE)
    log(f"clone {filepath} repo => {'error' if completed_process.returncode else 'cloned'}")
    if completed_process.returncode == 0:
        commit_hash = get_current_hash()
        if commit_hash is not None:
            save_last_hash(commit_hash)
    else:
        raise Exception(completed_process.stdout.decode())

    return True


def prepare_and_copy(pname, stats_json, upload_dir: str, unpack_dir: str, want_pull: bool):
    huge_list = []
    if os.path.exists(env.PP_CONFIG_HOW_TO_UNZIP(pname)):
        config = json.load(open(env.PP_CONFIG_HOW_TO_UNZIP(pname)))
    else:
        config = {"uploaded_files": {}}
    # {
    #   "uploaded_files": {
    #    "a.zip": { "which_set": "train", "to_db": True },
    #    "b.zip": { "which_set": "train", "to_db": False },
    #   }
    # }

    os.makedirs(env.PP_DIR_UNPACKED(pname), exist_ok=True)
    all_filenames = set(chain(os.listdir(upload_dir), os.listdir(unpack_dir)))
    for file_n, filename in enumerate(sorted(all_filenames)):
        if filename.startswith("."):
            continue
        upload_filename = os.path.join(upload_dir, filename)
        unpack_filename = os.path.join(unpack_dir, filename)
        files_found_jsonl = os.path.join(unpack_dir, filename, "_files_found.jsonl")
        if os.path.isfile(unpack_filename):
            continue

        stats_json["filtering_progress"] = int(100 * file_n / len(all_filenames))
        if want_pull:
            stats_json["uploaded_files"][filename] = {"status": "working"}
            stats_save(pname)

        if not os.path.exists(upload_filename):
            if os.path.isdir(unpack_filename):
                log(f"{filename} no longer exists => erase")
                cmd = ["rm", "-rf", unpack_filename]
                log(" ".join(cmd))
                subprocess.check_call(cmd)
            continue

        source_type = get_source_type(upload_filename)

        msg = None
        success = True
        need_force_rescan = False
        if source_type == "git":
            try:
                need_force_rescan = _prepare_git_repo(upload_filename, want_pull)
            except Exception as e:
                log("ERROR: %s" % (e or str(type(e))))
                msg = str(e)
                success = False
        if success:
            def mark_which_set(lst, subdir):
                which_set = config["uploaded_files"].get(subdir, {}).get("which_set", "train")
                to_db = config["uploaded_files"].get(subdir, {}).get("to_db", False)
                log("marking %i files from %s to which_set=\"%s\", to_db=%s" % (len(lst), filename, which_set, to_db))
                for x in lst:
                    x["subdir"] = subdir
                    x["which_set"] = which_set
                    x["to_db"] = to_db

            src_mtime = os.path.getmtime(upload_filename)
            ff_mtime = os.path.exists(files_found_jsonl) and os.path.getmtime(files_found_jsonl)
            if not ff_mtime or src_mtime > ff_mtime or need_force_rescan:
                log("mtime of %s = %s" % (upload_filename, time.ctime(src_mtime)))
                log("mtime of %s = %s" % (files_found_jsonl, time.ctime(ff_mtime)))
                log("need_force_rescan = %s" % need_force_rescan)
                log(f"{filename} needs update => copy, unpack, find files")
            else:
                log("reusing existing file list %s" % files_found_jsonl)
                extracted_files = list(jsonlines.open(files_found_jsonl))
                stats_json["uploaded_files"][filename] = {
                    "status": "completed",
                    **list_of_files_to_stats(extracted_files),
                }
                mark_which_set(extracted_files, filename)
                huge_list.extend(extracted_files)
                continue

            try:
                stats_json["uploaded_files"][filename] = {"status": "working"}
                stats_save(pname)
                rm_and_unpack(pname, upload_filename, unpack_filename, source_type, filename)
                assert os.path.isdir(unpack_filename)
                success, extracted_files = process_files_in_single_subdir(pname, stats_json, config, filename)
                if success:
                    stats_json["uploaded_files"][filename] = {
                        "status": "completed",
                        **list_of_files_to_stats(extracted_files),
                    }
                    stats_save(pname)
                    with open(files_found_jsonl, "w") as f:
                        for x in extracted_files:
                            f.write(json.dumps(x) + "\n")
                    mark_which_set(extracted_files, filename)
                    huge_list.extend(extracted_files)
            except BaseException as e:
                log("ERROR: %s" % (e or str(type(e))))
                raise
        stats_json["uploaded_files"][filename]["status"] = "completed" if success else "failed"
        if msg is not None:
            stats_json["uploaded_files"][filename]["message"] = msg
        stats_save(pname)

    log("total files %i" % len(huge_list))
    return huge_list


def rm_and_unpack(pname, upload_filename, unpack_filename, source_type, filename):
    cmd = ["rm", "-rf", unpack_filename]
    log(" ".join(cmd))
    subprocess.check_call(cmd)

    if source_type in ["archive", "singlefile"]:
        cmd = ["mkdir", "-p", unpack_filename]
        log(" ".join(cmd))
        subprocess.check_call(cmd)
        if source_type == "archive":
            if filename.endswith(".tar.gz"):
                cmd = ["tar", "-xzf", upload_filename, "-C", unpack_filename]
            elif filename.endswith(".bz2"):
                cmd = ["tar", "-xjf", upload_filename, "-C", unpack_filename]
            elif filename.endswith(".zip"):
                cmd = ["unzip", "-q", "-o", upload_filename, "-d", unpack_filename]
            elif filename.endswith(".tar"):
                cmd = ["tar", "-xf", upload_filename, "-C", unpack_filename]
            else:
                raise ValueError(f"unknown archive type for {filename} => skip")
        else:
            cmd = ["cp", upload_filename, unpack_filename]
    elif source_type == "git":
        cmd = ["cp", "-r", os.path.join(upload_filename, "sources"), unpack_filename]
    log(" ".join(cmd))
    subprocess.check_call(cmd)


def process_files_in_single_subdir(pname, stats_json, config, subdir):
    subdir_full = os.path.join(env.PP_DIR_UNPACKED(pname), subdir)
    try:
        files = ls_with_linguist(pname, subdir_full)
        result = []
        for i, x in enumerate(files):
            x["subdir"] = subdir
            result.append(x)
            if i % 100 == 0:
                stats_json["uploaded_files"][subdir]["files"] = i
                stats_save(pname)
    except BrokenPipeError:
        raise ValueError("Linguist doesn't work, make sure you've installed it from https://github.com/smallcloudai/linguist")
    return True, result


def list_of_files_to_stats(files):
    stat_error = 0
    stat_large = 0
    stat_generated = 0
    stat_vendored = 0
    stat_good = 0
    for i, x in enumerate(files):
        if "error" in x:
            stat_error += 1
            log("Error: %s" % x["error"])
            continue
        if x["large"]:
            stat_large += 1
            continue
        if x["generated"]:
            stat_generated += 1
            continue
        if x["vendored"]:
            stat_vendored += 1
            continue
        stat_good += 1
    log("stats: %i good, %i too large, %i generated, %i vendored" %
        (stat_good, stat_large, stat_generated, stat_vendored)
        )
    return {
        "files": len(files),
        "large": stat_large,
        "generated": stat_generated,
        "vendored": stat_vendored,
        "good": stat_good,
        "cant_read": stat_error,
    }


def dedup(pname, huge_list):
    """
    Deduplicate huge list of files.
    """
    log("dedup...")
    unique_by_namesize = dict()
    huge_filtered = []
    stats_json["files_before_dedup"] = len(huge_list)
    dups = []
    for n, x in enumerate(huge_list):
        if "error" in x:
            continue
        path = x["path"]
        name = os.path.basename(path)
        namesize = "%s:%i" % (name, x["lines"] // 10)
        dup = unique_by_namesize.get(namesize)
        if dup:
            is_test = dup["which_set"] == "test" or x["which_set"] == "test"
            is_train = dup["which_set"] == "train" or x["which_set"] == "train"
            which_set = x["which_set"]
            lang = x["language"]
            if lang is None:
                is_test = False
                is_train = False
                which_set = ""
            if is_train or is_test:
                which_set = "test" if is_test else "train"  # test overrides train
            to_db = dup["to_db"] or x["to_db"]
            dup["to_db"] = to_db
            dup["which_set"] = which_set
            dups.append(x)
            continue
        huge_filtered.append(x)
        stats_json["files_after_dedup"] = len(huge_filtered)
        unique_by_namesize[namesize] = x
        if n % 100 == 0:
            stats_save(pname)
    stats_save(pname)
    log("after dedup %i files" % len(huge_filtered))
    return huge_filtered, dups


def make_matcher(raw_masks: str):
    def cleanup_mask(m):
        res = m
        for sym in [' ', '\t']:
            res = ''.join(filter(lambda x: len(x) > 0, res.split(sym)))

        return res

    masks = [cleanup_mask(m) for m in raw_masks.split(',')]

    def matcher(source: str):
        if len(masks) > 0:
            return any([fnmatch(source, m) for m in masks])
        return None

    return matcher, len(masks) > 0


def save_into_sets(pname, records: List[Dict[str, Any]], dups):
    # Convert relative paths to absolute paths and validate it
    for record in records:
        filename = os.path.join(CWD, record["path"])
        assert filename.startswith(env.PP_DIR_UNPACKED(pname)), f'"{filename}" does not start with "{env.PP_DIR_UNPACKED(pname)}"'
        filename = filename[len(env.PP_DIR_UNPACKED(pname)):].lstrip("/")
        record["path"] = filename

    def _file_type(record: Dict[str, Any]) -> str:
        return record["language"] or record["mime_type"]

    def _filtered(record: Dict[str, Any]) -> bool:
        return "reason" not in record

    # file types with at least one filtered file or
    # rejected with filetypes_finetune filter
    suitable_to_train = set()
    digits_percent_max = 0.3
    for record in records:
        assert "reason" not in record

        if "error" in record:
            record["reason"] = "LINGUIST_ERROR"
        elif record["type"] != "Text":
            record["reason"] = "NOT_TEXT"
        elif not record["language"]:
            record["reason"] = "NOT_CODE"
        elif record["large"]:
            record["reason"] = "TOO_LARGE"
        elif record["generated"]:
            record["reason"] = "GENERATED"
        elif record["vendored"]:
            record["reason"] = "VENDORED"
        elif record["digits_percent"] > digits_percent_max:
            record["reason"] = "LOT_OF_DIGITS"
        else:
            suitable_to_train.add(_file_type(record))

    # filter config
    if not os.path.exists(env.PP_CONFIG_HOW_TO_FILETYPES(pname)):
        def _desc_sorting_key(item: Tuple[str, int]) -> Tuple[int, str]:
            return -item[1], item[0]

        filetypes_whitelist = {
            file_type
            for file_type, _ in sorted(
                Counter(filter(lambda t: t in TRUSTED_LANGUAGES, map(_file_type, records))).items(),
                key=_desc_sorting_key)[:1]
        }

        fcfg = {
            "filetypes_finetune": {
                file_type: True
                for file_type in filetypes_whitelist
            },
            "filetypes_db": {},
            "force_include": "",
            "force_exclude": "",
        }

        with open(env.PP_CONFIG_HOW_TO_FILETYPES(pname), "w") as f:
            json.dump(fcfg, f, indent=4)

    log("Reading %s" % env.PP_CONFIG_HOW_TO_FILETYPES(pname))
    with open(env.PP_CONFIG_HOW_TO_FILETYPES(pname), "r") as f:
        fcfg = json.load(f)

    # filter records
    filetypes_finetune = fcfg.get("filetypes_finetune", {})
    force_include_matcher, have_include_filters = make_matcher(fcfg.get('force_include', ''))
    force_exclude_matcher, _ = make_matcher(fcfg.get('force_exclude', ''))

    for record in records:
        if _filtered(record):
            if force_exclude_matcher(record['path']):
                suitable_to_train.add(_file_type(record))
                record["reason"] = "EXCLUDED_BY_MASK"
            elif not filetypes_finetune.get(_file_type(record), False):
                suitable_to_train.add(_file_type(record))
                record["reason"] = "TYPE_OFF"
        if force_include_matcher(record['path']):
            record.pop('reason', None)

    # construct sets
    to_train = list(filter(lambda r: _filtered(r) and r["which_set"] == "train", records))
    to_test = list(filter(lambda r: _filtered(r) and r["which_set"] == "test", records))
    to_db = list(filter(lambda r: _filtered(r) and r["to_db"], records))
    rejected = list(filter(lambda r: not _filtered(r), records))
    new_dups = []

    if have_include_filters:
        for record in dups:
            if force_include_matcher(record['path']):
                if _filtered(record) and record["which_set"] == "train":
                    to_train.append(record)
                elif _filtered(record) and record["which_set"] == "test":
                    to_test.append(record)
                else:
                    new_dups.append(record)
            else:
                new_dups.append(record)

    # write finetune records
    with open(env.PP_LOG_FILES_ACCEPTED_SCAN(pname), "w") as f:
        for x in to_train:
            f.write("FINETUNE %s\n" % x["path"])
        save_jsonl_if_changed(os.path.join(env.PP_DIR_UNPACKED(pname), "train_set.jsonl"), to_train)
        for x in to_test:
            f.write("MARKED AS TEST SET %s\n" % x["path"])
        save_jsonl_if_changed(os.path.join(env.PP_DIR_UNPACKED(pname), "test_set.jsonl"), to_test)

    # write filtered records
    with open(env.PP_LOG_FILES_REJECTED_SCAN(pname), "w") as f:
        for r in new_dups:
            p = r['path']
            p = os.path.join(CWD, p)
            assert p.startswith(env.PP_DIR_UNPACKED(pname))
            p = p[len(env.PP_DIR_UNPACKED(pname)):].lstrip("/")
            f.write("DUPLICATE %s\n" % p)
        for x in rejected:
            f.write(x["reason"] + " " + x["path"] + "\n")

    # write db records
    save_jsonl_if_changed(os.path.join(env.PP_DIR_UNPACKED(pname), "database_set.jsonl"), to_db)

    # update stats
    stats_json["filestats_scan_finetune"]["accepted"] = len(to_train) + len(to_test)
    stats_json["filestats_scan_finetune"]["rejected"] = len(rejected)
    stats_json["filestats_scan_db"]["accepted"] = len(to_db)
    stats_json["filestats_scan_db"]["rejected"] = len(rejected)
    stats_json["mime_types"] = list(sorted([
        {
            "file_type": file_type,
            "count": count,
            "suitable_to_train": file_type in suitable_to_train,
            "trusted_language": file_type in TRUSTED_LANGUAGES,
        }
        for file_type, count in Counter(map(_file_type, records)).items()
    ], key=lambda x: (not x["suitable_to_train"], x["file_type"])))


def save_jsonl_if_changed(fn, a_list):
    new_text = "".join((json.dumps(x) + "\n") for x in a_list)
    if os.path.isfile(fn):
        old_text = open(fn).read()
    else:
        old_text = "does not exist"
    if old_text == new_text:
        log("Will not overwrite '%s' because it is exactly the same as the current output" % fn)
        return
    log("Writing '%s'" % fn)
    with open(fn, "w") as f:
        f.write(new_text)


# "filter/file.py": {
#     "lines": 163,
#     "sloc": 145,
#     "type": "Text",
#     "mime_type": "application/x-python",
#     "language": "Python",
#     "large": false,
#     "generated": false,
#     "vendored": false
# }

@click.command()
@click.option("--pname", default="project1", help="Project name")
@click.option("--want-pull", is_flag=True, default=False, help="Run git pull before filtering")
def main(pname: str, want_pull: bool):
    stats_json["filtering_progress"] = 0
    stats_json["scan_status"] = "working"
    log("locking project '%s'" % pname)
    with filelock.FileLock(env.PP_PROJECT_LOCK(pname)):
        log("locked project '%s' successfully" % pname)
        stats_save(pname)  # saves CONFIG_PROCESSING_STATS
        try:
            huge_list = prepare_and_copy(pname, stats_json, env.PP_DIR_UPLOADS(pname), env.PP_DIR_UNPACKED(pname), want_pull)
            stats_json["filtering_progress"] = 100
            stats_save(pname)
            huge_list, dups = dedup(pname, huge_list)
            save_into_sets(pname, huge_list, dups)
            stats_json["scan_status"] = "finished"
            stats_json["scan_finished"] = True
            stats_json["scan_finished_ts"] = time.time()
            stats_save(pname)
        except Exception as e:
            stats_json["scan_status"] = "failed"
            stats_json["scan_error"] = str(e) or str(type(e))
            stats_save(pname)
            raise


if __name__ == '__main__':
    # logging.basicConfig(
    #     level=logging.INFO,
    #     format='%(asctime)s PREPROC %(message)s',
    #     datefmt='%Y%m%d %H:%M:%S',
    #     handlers=[logging.StreamHandler(stream=sys.stderr)])
    # YMD_hms = os.environ.get("LORA_LOGDIR", "")
    # if YMD_hms:
    #     traces.configure(task_dir="loras", task_name=YMD_hms, work_dir=env.PERMDIR)
    main()
