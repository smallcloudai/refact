import os
import torch as th
import termcolor
import time
import torch.distributed as dist

from refact_encoding import RefactEncoding, hlprint

from typing import Callable, Union, List, Set, Dict, Any, Optional

from refact_scratchpads.scratchpad_utils import temperature_top_k_top_p_filtering


DEBUGLOG_TOP3 = int(os.environ.get("DEBUG", "0"))


class ScratchpadBase:
    def __init__(
        self,
        enc: RefactEncoding,
        id: str,
        created: float,
        temperature: float,
        max_tokens: int,
        stop_tokens: Union[str, List[str]],
        logger: Callable,
        stream: bool = False,
        **unused,
    ):
        self.enc = enc
        self.id = id
        self._logger = logger
        self.created = created
        self.finish_reason = ""
        self.temp = min(max(float(temperature), 0.0), 1.0)
        self.max_tokens = int(max_tokens)
        tmp = stop_tokens
        if isinstance(tmp, str):
            stop_strings = [tmp]
        else:
            stop_strings = tmp
        self.stop_tokens: Set[int] = set()
        self.stop_lf = False
        self.stop_lf_lf = False
        self.stop_lf_lf_lf = False
        self.stream = stream
        for s in stop_strings:
            if s == "\n":
                self.stop_lf = True
                continue
            if s == "\n\n":
                self.stop_lf_lf = True
                continue
            if s == "\n\n\n":
                self.stop_lf_lf_lf = True
                continue
            t = self.enc.encode(s)
            if len(t) == 1:
                self.stop_tokens.add(t[0])
            else:
                self.debuglog("ScratchpadBase: cannot use '%s' as a stop token" % (s.replace("\n", "\\n")))
        for k, v in unused.items():
            self.debuglog("ScratchpadBase: unused parameter '%s' = '%s'" % (k, v))
        self.generated_tokens_n = 0
        self.needs_upload = False

    def before_token_selection(self, m, **kwargs) -> Dict[str, Any]:
        raise NotImplementedError()

    def select_tokens(
            self,
            logits: th.Tensor,
            tokens: th.Tensor,
            chosen_tokens: th.Tensor,
            *,
            temperatures: th.Tensor,
            logits_intrusion: Optional[List[Dict[int, float]]] = None,
            top_ps: Optional[List[float]] = None,
            top_ks: Optional[List[int]] = None,
            model_parallel_group: Optional[dist.ProcessGroup] = None,
            **unused
    ):
        if logits_intrusion:
            for idx, intr in enumerate(logits_intrusion):
                for t, add in intr.items():
                    if DEBUGLOG_TOP3:
                        self.debuglog("logit for %s is %0.3f, adding %0.3f" % (
                            hlprint(self.enc, [t]),
                            logits[idx, -1, t],
                            add))
                    logits[idx, -1, t] += add

        if top_ps is not None and top_ks is not None:
            for b in range(logits.shape[0]):
                logits[b, -1] = temperature_top_k_top_p_filtering(
                    logits[b, -1], temperature=temperatures[b],
                    top_p=top_ps[b], top_k=top_ks[b]
                )
            probs = logits[:, [-1]].softmax(dim=-1)
            a = th.multinomial(probs, num_samples=1)
        else:
            probs = (logits[:, [-1]] / (temperatures + 0.01)).squeeze(1).softmax(dim=-1)
            a = th.multinomial(probs, num_samples=1)

        if model_parallel_group is not None:
            dist.broadcast(a, src=0, group=model_parallel_group)

        tokens.copy_(a, non_blocking=True)
        chosen_tokens.copy_(tokens, non_blocking=True)

        result = dict(
            selected_tokens=tokens,
            selected_probs=probs
        )
        if DEBUGLOG_TOP3:
            result["top3"] = self._log_top3(token=tokens[0], probs=probs[0])
        return result

    def after_token_selection(self, m, **kwargs) -> Dict[str, Any]:
        raise NotImplementedError()

    def toplevel_fields(self):
        return {}

    def prompt(self, T: int):
        raise NotImplementedError()

    def completion(self, final: bool):
        raise NotImplementedError()

    def set_model_thresholds(self, **args):
        if len(args) > 0:
            self.debuglog("set_model_thresholds: unused parameters %s" % str(args))

    def debuglog(self, *args):
        elapsed = time.time() - self.created
        self._logger("%4.0fms" % (elapsed * 1000,), *args)

    def _log_top3(
            self,
            token: th.Tensor,
            probs: th.Tensor,
    ):
        def _format(t: str, color: str):
            return "\"%s\"" % termcolor.colored(t.replace("\n", "\\n").replace("\r", "\\r"), color)

        text = _format(self.enc.decode([token.item()]), "green").ljust(25)
        text += " <= "
        probs3, top3idx = map(lambda x: x.ravel().cpu().numpy(), probs.topk(4))
        for p, i in zip(probs3, top3idx):
            text += " %i %s" % (i, _format(self.enc.decode([i]), "yellow" if token.item() != i else "green"))
            text += " %0.1f%%" % (100 * p)
        return text

    def dump(self) -> bytes:
        import pickle
        enc = self.enc
        self.enc = None
        d = pickle.dumps(self)
        self.enc = enc
        return d

    def set_enc(self, enc):
        self.enc = enc
