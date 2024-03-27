from collections import defaultdict
from typing import Iterator, Tuple, Dict, Any, Callable, Iterable, List

import torch as th
import torch.distributed as dist

from refact_data_pipeline import DatasetOpts
from refact_data_pipeline.pipeline_pieces import PipelineNode


def str2dtype(s: str) -> th.dtype:
    assert isinstance(s, str)
    return {
        "torch.bfloat16": th.bfloat16,
        "torch.float16": th.float16,
        "torch.float32": th.float32,
        "torch.int64": th.int64,
        "torch.bool": th.bool,
    }[s]


_prefer_dtypes = {
    "logits": th.int64,
    "first": th.bool,
    "mask": th.bool
}


def _after_collate(result: Dict[str, th.Tensor]) -> Dict[str, th.Tensor]:
    if 'first' in result:
        result['first'] = result.pop("first")[:, :-1]
    if 'mask' in result:
        result['mask'] = result.pop("mask")[:, 1:]
    result["labels"] = result["tokens"][:, 1:]
    result["input"] = result["tokens"][:, :-1]
    return {
        k: (v if isinstance(v, th.Tensor) else v)
        for k, v in result.items()
    }


def collate_fn(records: List[Dict[str, Any]]) -> Dict[str, Any]:
    output = defaultdict(list)
    last_stats = None
    for idx, record in enumerate(records):
        for k, v in record.items():
            if k == "stats":
                last_stats = v
                continue
            output[k].append(
                th.tensor(record[k], dtype=_prefer_dtypes.get(k, th.int64))
            )
    return _after_collate({
        "stats": last_stats,
        **{k: th.stack(v).contiguous() for k, v in output.items()}
    })


def data_parallel_split_and_collate_fn(records: List[Dict[str, Any]], global_batch_size: int) -> Dict[str, Any]:
    rank = dist.get_rank()
    world_size = dist.get_world_size()
    effective_bs = global_batch_size // world_size
    assert effective_bs * world_size == len(records), "effective batch size %s" % len(records)

    output = defaultdict(list)
    last_stats = None
    for idx, record in enumerate(records):
        for k, v in record.items():
            if k == "stats":
                last_stats = v
                continue
            output[k].append(
                th.tensor(record[k], dtype=_prefer_dtypes.get(k, th.int64))
            )

    from_, to = rank * effective_bs, (rank + 1) * effective_bs
    return _after_collate({
        "stats": last_stats,
        **{k: th.stack(v)[from_:to].contiguous() for k, v in output.items()}
    })


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


class BatchIterator(PipelineNode):
    def __init__(
            self,
            inner_filter: Iterable[Any],
            dataopts: DatasetOpts
    ):
        super().__init__(dataopts)
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.batch_size = dataopts.get("batch_size", 1)
        self.device = dataopts.get("device", "cuda")
        self.drop_last = dataopts.get("drop_last", False)

    def __iter__(self):
        seq_iter = iter(self.inner_filter)
        while True:
            data, datastats = read_and_collate(
                data_iter=seq_iter,
                prefer_dtypes=dict(mask='torch.bool', first='torch.bool'),
                B=self.batch_size,
                device=self.device,
                cold_restart_dict=dict(),
                log_stats=True,
                progress_callback=lambda *args, **kwargs: None
            )
            if len(data) == 0:
                break

            if self.drop_last and len(data['tokens']) < self.batch_size:
                break

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
            yield batch, datastats
