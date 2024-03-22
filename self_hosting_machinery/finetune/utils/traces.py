import datetime
import json
import logging
import os
import sys
import traceback

import torch.distributed as dist

from collections import defaultdict
from dataclasses import dataclass, field

from typing import Dict, Optional, TextIO, Any, List

_cx: Optional['TraceContext'] = None


def p(tensor) -> str:
    return "*".join(["%i" % i for i in tensor.shape]) \
        + " " + str(tensor.dtype).replace("torch.", "")


class MyLogHandler(logging.Handler):
    def emit(self, record):
        timestamp = datetime.datetime.now().strftime("%Y%m%d %H:%M:%S")
        try:
            sys.stderr.write(timestamp + " " + self.format(record) + "\n")
            sys.stderr.flush()
        except BrokenPipeError:
            # happens sometimes when one of multi-GPU processes is killed
            pass


handler = MyLogHandler()
handler.setLevel(logging.INFO)
handler.setFormatter(logging.Formatter("FTUNE %(message)s"))
root = logging.getLogger()
root.addHandler(handler)
root.setLevel(logging.INFO)


@dataclass
class TraceContext:
    task_dir: str
    task_name: str
    path: str
    unique_id: str
    log_fn: str
    console_logger: Optional[TextIO] = None
    progress: Optional[TextIO] = None
    name2val: Dict[str, List[float]] = field(default_factory=lambda: defaultdict(list))
    step: Optional[int] = None


def context() -> Optional[TraceContext]:
    return _cx


def configure(
        *,
        task_dir: str = "",
        task_name: str = "",
        work_dir: str = "",
):
    def _except_hook(exctype, value, tb):
        import socket
        host = socket.gethostname()
        log(
            "\n%s Caught exception:\n" % host + "".join(
                traceback.format_exception(exctype, value, tb, limit=None, chain=True))
        )
        quit(1)

    sys.excepthook = _except_hook
    # More messages could be found in
    # /home/user/.local/lib/python3.9/site-packages/deepspeed/ops/op_builder/builder.py:445 (verbose is True)
    logging.getLogger("DeepSpeed").setLevel(logging.WARNING)

    if task_name == "NO_LOGS":
        return

    global _cx
    assert _cx is None
    path = os.path.join(work_dir, task_dir, task_name)
    os.makedirs(path, exist_ok=True)
    _cx = TraceContext(
        task_dir=task_dir,
        task_name=task_name,
        path=path,
        unique_id=task_dir + "-" + task_name,
        console_logger=sys.stdout,
        progress=None,
        log_fn=os.path.join(path, "log.txt"),
    )


def progress(key: str, val: Any) -> None:
    import torch
    assert _cx is not None, "Call configure() first"
    if isinstance(val, (float, int)):
        _cx.name2val[key].append(val)
    elif isinstance(val, torch.Tensor):
        _cx.name2val[key].append(val.item())
    else:
        raise NotImplementedError


def progress_dump(
        step: int,
        ignore_list: List[str] = [],
):
    assert _cx is not None, "Call configure() first"
    _cx.step = step
    avg = {name: float(sum(vals) / len(vals)) for name, vals in _cx.name2val.items()}
    _cx.name2val.clear()

    if _cx.progress is None:
        _cx.progress = open(_cx.path + "/progress.jsonl", "w")
    _cx.progress.write(json.dumps(avg) + "\n")
    _cx.progress.flush()

    if len(avg) == 0:
        return avg

    out = [(k, "%-8.3g" % v) for k, v in avg.items()]
    kwidth = max(map(lambda x: len(x[0]), out))
    vwidth = max(map(lambda x: len(x[1]), out))
    msg = ["-" * (kwidth + vwidth + 7)]
    for k, v in out:
        if [1 for ignore in ignore_list if k.startswith(ignore)]:
            continue
        msg.append(
            "| %s%s | %s%s |"
            % (k, " " * (kwidth - len(k)), v, " " * (vwidth - len(v)))
        )
    msg.append(msg[0])
    if len(msg) > 2:
        log("\n".join(msg))
    return avg


def log(*args) -> None:
    if dist.is_initialized() and dist.get_rank() != 0:
        return

    s = " ".join(map(str, args))
    if not _cx:
        # not configured, will work as print to stderr
        sys.stderr.write(" ".join(map(str, args)) + "\n")
        sys.stderr.flush()
        return
    if _cx.console_logger:
        _cx.console_logger.write(s + "\n")
        _cx.console_logger.flush()
    with open(_cx.log_fn, "a", encoding='utf-8') as f:
        f.write(s + "\n")


def touch() -> None:
    if _cx:
        os.utime(_cx.path)
