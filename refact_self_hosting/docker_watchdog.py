import time, sys, os, subprocess, signal, requests, json
from typing import List, Optional, Dict


LOGDIR = os.path.expanduser("~/perm-storage/logs-watchdog")
CONFIGDIR = os.path.expanduser("~/perm-storage/cfg/watchdog.d")


def log(*args):
    msg = " ".join(map(str, args))
    sys.stderr.write(msg + "\n")
    sys.stderr.flush()
    date = time.strftime("%Y%m%d")
    with open(os.path.join(LOGDIR, "%s_inf_watchdog.log" % date), "a") as f:
        f.write(msg + "\n")


compile_required = set()
compile_successful = set()
compiling_now = ""


class TrackedJob:
    def __init__(self, cfg):
        self.p: Optional[subprocess.Popen] = None
        self.cmdline_str = " ".join(cfg["command_line"])
        self.start_ts = 0
        self.cfg = cfg
        self.please_shutdown = False
        self.remove_this = False
        self.sent_sigusr1_ts = 0

    def start(self):
        if self.p is not None:
            return
        global compile_required, compiling_now
        alt_env = os.environ.copy()
        CUDA_VISIBLE_DEVICES = ",".join(["%d" % x for x in self.cfg["gpus"]])
        cmdline = list(self.cfg["command_line"])
        if self.cfg.get("needs_compile", False):
            compile_required.add(self.cmdline_str)
            if compiling_now:
                return
            if self.cmdline_str not in compile_successful:
                compiling_now = self.cmdline_str
                cmdline.append("--compile")
        alt_env["CUDA_VISIBLE_DEVICES"] = CUDA_VISIBLE_DEVICES
        log("%s CVD=%s starting %s" % (time.strftime("%Y%m%d %H:%M:%S"), CUDA_VISIBLE_DEVICES, self.cmdline_str))
        self.start_ts = time.time()
        self.p = subprocess.Popen(
            cmdline,
            env=alt_env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        log("%s pid=%s" % (time.strftime("%Y%m%d %H:%M:%S"), self.p.pid))
        os.set_blocking(self.p.stderr.fileno(), False)

    def maybe_can_start(self):
        if self.p is not None:
            return
        if self.please_shutdown:
            return
        policy = self.cfg.get("policy", [])
        assert set(policy) <= {"always_on", "when_file_appears", "at_night", "always_on_low_priority"}, policy
        if "when_file_appears" in policy:
            the_file = self.cfg["when_file_appears"]
            if the_file.startswith("~"):
                the_file = os.path.expanduser(the_file)
            if os.path.exists(the_file):
                os.remove(the_file)
                self.start()
        elif "always_on" in policy:
            self.start()
        elif "always_on_low_priority" in policy:
            self.start()
        elif "at_night" in policy:
            pass

    def poll_logs(self) -> bool:
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
                "ninja",
                "Detected CUDA files",
                "PyTorch extensions root",
                "RequestsDependencyWarning",
                "warnings.warn(\"urllib3",
            ]:
                if test in line:
                    garbage = True
                    break
            if not garbage:
                log("-- %s -- %s" % (self.p.pid, line))
        if self.p.poll() is not None:
            retcode = self.p.returncode
            if retcode:
                log("%s crashed %s, retcode %i" % (time.strftime("%Y%m%d %H:%M:%S"), self.cmdline_str, retcode))
            else:
                log("%s finished %s" % (time.strftime("%Y%m%d %H:%M:%S"), self.cmdline_str))
            if self.cmdline_str == compiling_now:
                log("/finished compiling as recognized by watchdog")
                compiling_now = None
                if retcode == 0:
                    compile_successful.add(self.cmdline_str)
            self.p.communicate()
            self.p = None
            self.start_ts = 0
            self.sent_sigusr1_ts = 0
            self.please_shutdown = False
        return not self.p

    def maybe_needs_restart(self):
        restart_every = self.cfg.get("restart_every", 0)
        if not restart_every:
            return
        now = time.time()
        if now - self.start_ts > restart_every:
            self.please_shutdown = True

    def maybe_send_usr1(self):
        if not self.p:
            return
        if self.please_shutdown:
            self.p.send_signal(signal.SIGUSR1)
            self.sent_sigusr1_ts = time.time()
        if self.please_shutdown and self.sent_sigusr1_ts > time.time() + 30:
            self.p.kill()


tracked: Dict[str, TrackedJob] = {}


def create_tracked_jobs_from_configs():
    now_missing = set(tracked.keys())
    for fn in os.listdir(CONFIGDIR):
        if not fn.endswith(".cfg"):
            continue
        cfg = json.load(open(os.path.join(CONFIGDIR, fn)))
        for i in range(len(cfg["command_line"])):
            if cfg["command_line"][i].startswith("~"):
                cfg["command_line"][i] = os.path.expanduser(cfg["command_line"][i])
            if cfg["command_line"][i] == "python":
                cfg["command_line"][i] = sys.executable
        if fn in tracked:
            tracked[fn].cfg = cfg
        else:
            tracked[fn] = TrackedJob(cfg)
            log("%s adding job %s" % (time.strftime("%Y%m%d %H:%M:%S"), fn))
        now_missing.discard(fn)
    for fn in now_missing:
        tracked[fn].please_shutdown = True
        tracked[fn].remove_this = True


def main_loop():
    global quit_flag
    while 1:
        create_tracked_jobs_from_configs()
        for fn, job in tracked.items():
            job.maybe_can_start()
            job.maybe_needs_restart()
            job.maybe_send_usr1()
            dead = job.poll_logs()
            if dead and job.remove_this:
                log("%s removing %s" % (time.strftime("%Y%m%d %H:%M:%S"), fn))
                del tracked[fn]
                break
        time.sleep(1)


if __name__ == '__main__':
    os.makedirs(LOGDIR, exist_ok=True)
    main_loop()


# model = os.environ.get("SERVER_MODEL")
