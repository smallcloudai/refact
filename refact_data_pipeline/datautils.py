import torch as th
from collections import defaultdict
from typing import Iterator, Tuple, Dict, Any, Callable, Sequence


def str2dtype(s: str) -> th.dtype:
    assert isinstance(s, str)
    return {
        "torch.bfloat16": th.bfloat16,
        "torch.float16": th.float16,
        "torch.float32": th.float32,
        "torch.int64": th.int64,
        "torch.bool": th.bool,
    }[s]


def read_and_collate(
        data_iter: Iterator,
        prefer_dtypes: Dict[str, str],
        B: int,
        *,
        device: str,
        cold_restart_dict: Dict[str, int],
        log_stats: bool,
        progress_callback: Callable[[str, float], None],
) -> Tuple[Dict[str, th.Tensor], Dict[str, Any]]:
    output = defaultdict(list)
    for _ in range(B):
        rec = None
        try:
            rec = next(data_iter)
        except StopIteration:
            break
        if log_stats:
            for sk, sv in rec["stats"].items():
                if isinstance(sv, (float, int)) and not sk.startswith("restart"):
                    progress_callback("ds/%s" % sk, sv)
        for k, v in rec.items():
            if k == "stats":
                for sk, sv in v.items():
                    if sk.startswith("restart"):
                        cold_restart_dict[sk] = sv
                continue
            output[k].append(th.tensor(rec[k], dtype=str2dtype(prefer_dtypes.get(k, "torch.int64"))))
    lens = []
    for k in output:
        if k != "stats":
            lens.append(len(output[k]))
    if len(output) > 0:
        len0 = lens[0]
        assert all(l == len0 for l in lens), "all lengths must be equal %s" % lens
    return (
        {k: th.stack(v).to(device) for k, v in output.items()},
        rec["stats"] if rec is not None else {},
    )


class BatchIterator:
    def __init__(
            self,
            seq: Sequence,
            dataopts: Dict[str, Any],
    ):
        self.seq_iter = iter(seq)
        self.dataopts = dataopts
        self.batch_size = dataopts.get("batch_size", 1)
        self.device = dataopts.get("device", "cuda")
        self.drop_last = dataopts.get("drop_last", False)

    def __next__(self):
        data, datastats = read_and_collate(
            data_iter=self.seq_iter,
            prefer_dtypes=dict(mask='torch.bool', first='torch.bool'),
            B=self.batch_size,
            device=self.device,
            cold_restart_dict=dict(),
            log_stats=True,
            progress_callback=lambda *args, **kwargs: None
        )
        if len(data) == 0:
            raise StopIteration()

        if self.drop_last and len(data['tokens']) < self.batch_size:
            raise StopIteration()

        extra = dict()
        if 'first' in data:
            extra['first'] = data.pop("first")[:, :-1]
        if 'mask' in data:
            extra['mask'] = data.pop("mask")[:, 1:]

        tokens = data.pop("tokens")
        batch = dict(
            labels=tokens[:, 1:],
            input=tokens[:, :-1],
            **extra
        )
        batch.update({k: v for k, v in data.items() if k not in batch})
        return batch, datastats

    def __iter__(self):
        return self