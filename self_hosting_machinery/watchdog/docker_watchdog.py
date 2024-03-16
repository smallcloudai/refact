import json
import os
import shutil
import glob
import signal
import subprocess
import sys
import time
import uuid

from pathlib import Path

from typing import Dict, Optional, List

from refact_utils.scripts import env

FIRST_RUN_CMDLINE = [sys.executable, "-m", "self_hosting_machinery.scripts.first_run"]


def replace_variable_names_from_env(s):
    s = s.replace("%PYTHON%", sys.executable)
    for k, v in env.__dict__.items():
        if k.startswith("FLAG_") or k.startswith("DIR_") or k.startswith("CONFIG_"):
            s = s.replace("%" + k + "%", v)
    return s


log_prevdate = ""

def log(*args):
    msg = " ".join(map(str, args))
    sys.stderr.write(msg + "\n")
    sys.stderr.flush()
    date = time.strftime("%Y%m%d")
    global log_prevdate
    if log_prevdate != date:
        log_prevdate = date
        list_of_files = glob.glob(f'{env.DIR_LOGS}/watchdog_*.log')
        list_of_files.sort()
        while len(list_of_files) > 20:
            try:
                os.remove(list_of_files.pop(0))
            except OSError:
                pass
    with open(os.path.join(env.DIR_LOGS, "watchdog_%s.log" % date), "a") as f:
        f.write(msg + "\n")


compile_successful = set()
compile_unsuccessful = set()
compiling_now = ""


def cfg_to_cmdline(cfg):
    """
    This allows for tasks to move across GPUs, if gpu has changed, than command line changes for the purpose of
    restarting.
    """
    return " ".join(cfg["command_line"]) + " @"+ "".join(":gpu%02d" % x for x in cfg["gpus"])


def cfg_to_compile_key(cfg):
    """
    But to compile (download weights and build cuda kernels) the key should be the same regardless of gpu.
    """
    return " ".join(cfg["command_line"])


class TrackedJob:
    def __init__(self, cfg, cfg_filename: Path):
        self.p: Optional[subprocess.Popen] = None
        self.cmdline_str = cfg_to_cmdline(cfg)
        self.compile_str = cfg_to_compile_key(cfg)
        self.start_ts = 0
        self.cfg = cfg
        self.cfg_filename: Path = cfg_filename
        self.please_shutdown = False
        self.remove_this = False
        self.sent_sigusr1_ts = 0
        self.status_from_stderr = ""
        self.status_nickname = ""
        self.command_not_found = False

    def set_status(self, newstatus):
        self.status_from_stderr = newstatus
        save_status_fn = self.cfg.get("save_status", "")
        if save_status_fn:
            status_nickname = self.cfg["save_status_nickname"]
            self.status_nickname = status_nickname if not status_nickname.startswith("prog_") else status_nickname[5:]
            save_status_fn = replace_variable_names_from_env(save_status_fn)
            log("overwrite %s with prog=%s status=%s" % (save_status_fn, status_nickname, newstatus))
            with open(save_status_fn + ".tmp", "w") as f:
                f.write(json.dumps({
                    "prog": status_nickname if newstatus != "idle" else "",
                    "status": newstatus
                }))
            os.rename(save_status_fn + ".tmp", save_status_fn)

    def _start(self):
        if self.p is not None or self.command_not_found:
            return
        global compiling_now
        alt_env = os.environ.copy()
        cmdline = list(self.cfg["command_line"])
        if self.cfg.get("needs_compile", False):
            if compiling_now:
                return
            if self.compile_str in compile_unsuccessful:
                return
            if self.compile_str not in compile_successful:
                compiling_now = self.compile_str
                cmdline.append("--compile")
        self.set_status("starting")
        CUDA_VISIBLE_DEVICES = ",".join(["%d" % x for x in self.cfg["gpus"]])
        alt_env["CUDA_VISIBLE_DEVICES"] = CUDA_VISIBLE_DEVICES
        self.start_ts = time.time()
        try:
            self.p = subprocess.Popen(
                cmdline,
                env=alt_env,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
            )
        except FileNotFoundError as e:
            log(f"failed to run command {self.cmdline_str}: '{e}'")
            self.p = None
            self.command_not_found = True
            self.set_status("failed")
            return
        log("%s CVD=%s starting %s\n -> pid %s" % (
            time.strftime("%Y%m%d %H:%M:%S"),
            CUDA_VISIBLE_DEVICES,
            " ".join(cmdline),  # not self.cmdline_str so we can see "--compile"
            self.p.pid,
        ))
        os.set_blocking(self.p.stderr.fileno(), False)
        interrupt_when_file_appears = self.cfg.get("interrupt_when_file_appears", "")
        if interrupt_when_file_appears:
            p = replace_variable_names_from_env(interrupt_when_file_appears)
            if os.path.exists(p):
                os.unlink(p)

    def _poll_logs(self) -> bool:
        if self.p is None:
            return True
        global compiling_now
        while 1:
            line = self.p.stderr.readline()
            if not line:
                break
            line = line.decode("utf-8").rstrip()
            garbage = False
            for test in [
                "Loading extension module",
                "Building extension module",
                "ninja",
                "Detected CUDA files",
                "skipping build step",
                "PyTorch extensions root",
                "RequestsDependencyWarning",
                "warnings.warn(\"urllib3",
                "Positional args are being",
                "warnings.warn",
            ]:
                if test in line:
                    garbage = True
                    break
            if not garbage:
                log("-- %s -- %s" % (self.p.pid, line))
            if " STATUS " in line:
                cut_here = line.index(" STATUS ") + len(" STATUS ")
                self.set_status(line[cut_here:].strip())
        if self.p.poll() is not None:
            retcode = self.p.returncode
            log("%s %s finished %s, retcode %i" % (
                time.strftime("%Y%m%d %H:%M:%S"), self.p.pid, self.cmdline_str, retcode
            ))
            if retcode == 0:
                self.set_status("finished")
            elif retcode == 99:
                self.set_status("interrupted")
            else:
                self.set_status("failed")
            # retcode -10 is SIGUSR1
            if self.compile_str == compiling_now:
                compiling_now = None
                if retcode == 0:
                    log("/finished compiling as recognized by watchdog")
                    compile_successful.add(self.compile_str)
                elif self.sent_sigusr1_ts == 0:   # failed, and watchdog didn't try to kill it
                    log("/finished compiling -- failed, probably unrecoverable, will not retry")
                    compile_unsuccessful.add(self.compile_str)
            self.p.communicate()
            self.p = None
            self.sent_sigusr1_ts = 0
            self.please_shutdown = False
            policy = self.cfg.get("policy", [])
            if "single_shot" in policy:
                self.cfg_filename.unlink()
        return not self.p

    def maybe_needs_stop(self):
        if not self.p:
            return
        restart_every = self.cfg.get("restart_every", 0)
        if restart_every:
            now = time.time()
            if now - self.start_ts > restart_every:
                self.please_shutdown = True
        policy = self.cfg.get("policy", [])
        if "when_file_appears" in policy:
            # If the process is already running, prevent it from restarting again when it's over
            p = replace_variable_names_from_env(self.cfg["when_file_appears"])
            if os.path.exists(p):
                os.unlink(p)
        interrupt_when_file_appears = self.cfg.get("interrupt_when_file_appears", "")
        if interrupt_when_file_appears:
            p = replace_variable_names_from_env(interrupt_when_file_appears)
            if os.path.exists(p):
                self.please_shutdown = True
                os.unlink(p)

    def maybe_send_usr1(self, sigkill_timeout=30):
        if not self.p:
            self.please_shutdown = False  # this overrides True from "preempt" that sometimes happens (because of the task order)
            return
        if self.please_shutdown and self.sent_sigusr1_ts == 0:
            self.p.send_signal(signal.SIGUSR1)
            self.sent_sigusr1_ts = time.time()
        if self.please_shutdown and self.sent_sigusr1_ts + sigkill_timeout < time.time():
            log("%s SIGUSR1 timed out, sending kill %s" % (time.strftime("%Y%m%d %H:%M:%S"), self.p.pid))
            self.p.kill()

    def maybe_can_start(self):
        if self.p is not None:
            return
        if self.please_shutdown:
            return

        policy = self.cfg.get("policy", [])
        assert set(policy) <= {"always_on", "when_file_appears", "single_shot", "always_on_low_priority",
                               "periodic"}, policy
        if "when_file_appears" in policy:
            the_file = replace_variable_names_from_env(self.cfg["when_file_appears"])
            if os.path.exists(the_file):
                can_start = preempt_low_priority(self.cfg.get("gpus", []))
                if can_start:
                    os.remove(the_file)
                    self._start()
        elif "always_on" in policy:
            can_start = preempt_low_priority(self.cfg.get("gpus", []))
            if can_start:
                self._start()
        elif "always_on_low_priority" in policy:
            can_start = low_priority_can_start(self)
            if can_start:
                self._start()
        elif "single_shot" in policy:
            can_start = preempt_low_priority(self.cfg.get("gpus", []))
            if can_start:
                self._start()
        elif "periodic" in policy:
            if self.start_ts + self.cfg["restart_every"] < time.time():
                self._start()

    def __str__(self):
        return f"TrackedJob:\n" \
               f"  pid: {self.p.pid if self.p else None}\n" \
               f"  cmd: '{self.cmdline_str}'\n" \
               f"  start_ts: {self.start_ts}\n" \
               f"  cfg: {self.cfg}\n" \
               f"  shutdown: {self.please_shutdown}\n" \
               f"  remove: {self.remove_this}\n" \
               f"  status: {self.status_from_stderr}\n"


tracked: Dict[str, TrackedJob] = {}
watchdog_templates = list(Path(env.DIR_WATCHDOG_TEMPLATES).iterdir())


def create_tracked_jobs_from_configs():
    now_missing = set(tracked.keys())
    watchdog_configs = list(Path(env.DIR_WATCHDOG_D).iterdir())
    for filename in sorted(watchdog_configs + watchdog_templates):
        if not filename.name.endswith(".cfg"):
            continue
        fn = filename.name
        cfg = json.loads(filename.read_text())
        if cfg.get("unfinished", False):
            continue
        for i in range(len(cfg["command_line"])):
            cfg["command_line"][i] = replace_variable_names_from_env(cfg["command_line"][i])
        if fn in tracked:
            tracked[fn].cfg = cfg
            if tracked[fn].cmdline_str != cfg_to_cmdline(cfg) and not tracked[fn].remove_this:
                log("%s command line changed, stop job %s" % (time.strftime("%Y%m%d %H:%M:%S"), tracked[fn].cmdline_str))
                tracked[fn].please_shutdown = True
                tracked[fn].remove_this = True
        else:
            tracked[fn] = TrackedJob(cfg, cfg_filename=filename)
            log("%s adding job %s" % (time.strftime("%Y%m%d %H:%M:%S"), fn))
            tracked[fn].set_status("idle")
        now_missing.discard(fn)
    for fn in now_missing:
        tracked[fn].please_shutdown = True
        tracked[fn].remove_this = True


def preempt_low_priority(gpus):
    can_start = True
    for job in tracked.values():
        if "always_on_low_priority" not in job.cfg["policy"]:
            continue
        if set(gpus) & set(job.cfg["gpus"]):
            if job.p is not None:
                can_start = False
            if not job.please_shutdown:
                log("%s shutdown low priority job %s" % (time.strftime("%Y%m%d %H:%M:%S"), job.cmdline_str))
                job.please_shutdown = True
    return can_start


def low_priority_can_start(job: TrackedJob):
    can_start = True
    for tracked_job in tracked.values():
        if job.cfg.get("share_gpu", False) and tracked_job.cfg.get("share_gpu", False):
            continue
        if set(job.cfg.get("gpus", [])) & set(tracked_job.cfg.get("gpus", [])):
            if tracked_job.p is not None:
                can_start = False
    return can_start


_inform_about_gpu_status = ""


def inform_about_gpu_status():
    global _inform_about_gpu_status
    gpu_status: Dict[int, List[Dict]] = {}
    for job in tracked.values():
        if job.p is None:
            continue
        for gpu in map(int, job.cfg["gpus"]):
            if gpu >= 0:
                t = job.cmdline_str
                if t.startswith("python -m"):
                    t = t[len("python -m"):]
                status = []
                if job.status_nickname:
                    status = [job.status_nickname]
                gpu_status.setdefault(gpu, []).append({
                    "command": t.strip(),
                    "status": " ".join(status + [job.status_from_stderr]),
                })
    s = json.dumps({"gpus": gpu_status}, indent=4) + "\n"
    if s != _inform_about_gpu_status:
        with open(env.CONFIG_BUSY_GPUS + ".tmp", "w") as f:
            f.write(s)
        os.rename(env.CONFIG_BUSY_GPUS + ".tmp", env.CONFIG_BUSY_GPUS)
        _inform_about_gpu_status = s


def main_loop_body():
    try:
        create_tracked_jobs_from_configs()
    except FileNotFoundError:
        log("FileNotFoundError: might be this rare case: os.listdir() returned a file, but it was deleted before we could read it. Start over, this message should disappear next iteration.")
        return
    for fn, job in tracked.items():
        job.maybe_can_start()
        job.maybe_needs_stop()
        job.maybe_send_usr1()
        dead = job._poll_logs()
        if dead and job.remove_this:
            log("%s cleanup %s" % (time.strftime("%Y%m%d %H:%M:%S"), fn))
            del tracked[fn]
            break
    inform_about_gpu_status()


def shutdown_all():
    while len(tracked):
        for fn, job in tracked.items():
            job.please_shutdown = True
            job.maybe_send_usr1(sigkill_timeout=1)
            dead = job._poll_logs()
            if dead:
                log("%s cleanup %s" % (time.strftime("%Y%m%d %H:%M:%S"), fn))
                del tracked[fn]
                break
            else:
                log("%s still not dead %s" % (time.strftime("%Y%m%d %H:%M:%S"), fn))
        time.sleep(1)


def factory_reset():
    for todel in [
        env.DIR_LOGS,
        env.DIR_CONFIG,
        env.DIR_LORAS,
        env.DIR_SSH_KEYS,
        env.DIR_UPLOADS,
        env.DIR_WEIGHTS,
        env.DIR_PROJECTS,
    ]:
        try:
            shutil.rmtree(todel)
        except Exception as e:
            # not log, because no logs dir
            print("didn't delete %s: %s" % (todel, e))


def first_run():
    subprocess.check_call(FIRST_RUN_CMDLINE)


def main_loop():
    # Generate a random SMALLCLOUD_API_KEY, it will be inherited by subprocesses,
    # this allows inference_worker to authorize on the local web server (both use
    # this variable), and work safely even if we expose http port to the world.
    os.environ["SMALLCLOUD_API_KEY"] = str(uuid.uuid4())

    first_run()
    while 1:
        main_loop_body()
        time.sleep(1)
        if os.path.exists(env.FLAG_FACTORY_RESET):
            shutdown_all()
            log("\n * * * FACTORY RESET * * *\n")
            os.unlink(env.FLAG_FACTORY_RESET)   # if we can't delete it, that's an infinite loop, better to crash
            factory_reset()
            first_run()


if __name__ == '__main__':
    main_loop()
