from collections import deque
from typing import Optional, List

import torch
import torch.nn.functional as F

__all__ = ['masked_loss']

unmasked_avg_buf = None


def masked_loss(
        logits: torch.Tensor,
        labels: torch.Tensor,
        mask: Optional[torch.Tensor] = None,
        *,
        average_elements: int,
        enc: 'Encoding',
        debug_dump: Optional[List[str]] = None
) -> torch.Tensor:
    def _average(one_d_tensor: torch.Tensor) -> torch.Tensor:
        global unmasked_avg_buf
        if unmasked_avg_buf is None:
            unmasked_avg_buf = deque(maxlen=average_elements)
        if torch.is_grad_enabled():
            for x in one_d_tensor:
                unmasked_avg_buf.append(float(x))
            return sum(unmasked_avg_buf) / len(unmasked_avg_buf)
        else:
            return one_d_tensor.to(torch.float32).mean().item()

    _mb, _T = labels.shape
    mb, T, U = logits.shape
    assert _T == T
    assert _mb == mb

    ce = F.cross_entropy(
        logits.reshape(mb * T, U),
        labels.reshape(mb * T),
        reduction="none"
    ).reshape(mb, T)
    avg_mask_sum = _average(mask.sum(dim=1))
    loss_ce = ((ce * mask).sum(dim=1) / avg_mask_sum).mean()

    if debug_dump is not None:
        import termcolor
        def token_str(x, cond, color):
            t = "\"" + enc.decode([x]).replace("\n", "\\n") + "\""
            if cond:
                return termcolor.colored(t, color)
            else:
                return t

        with torch.no_grad():
            b = 0
            for ti in range(T):
                if b == -1:
                    continue
                if ti & 15 == 0:
                    debug_dump.append("-----")
                largest_logit_n = logits[b, ti].argmax().item()
                debug_dump.append(" ".join([
                    "%04i" % (ti,),
                    "ce=%5.2f" % ce[b, ti].item(),
                    "label=%-20s" % token_str(labels[b, ti].item(), mask[b, ti].item(), "green"),
                    "mask=%i" % mask[b, ti].item(),
                    "largest_logit=%05i" % largest_logit_n,
                    "modelthinks=%-10s" % token_str(largest_logit_n,
                                                    (mask[b, ti].item() and labels[b, ti].item() != largest_logit_n),
                                                    "red"),
                ]))
        debug_dump.append("-- (ce * mask).sum(dim=1) = %s" % (ce * mask).sum(dim=1))
        debug_dump.append("-- avg_mask_sum = %s" % avg_mask_sum)
        debug_dump.append("-- this example loss_ce = %5.3f" % loss_ce.item())

    return loss_ce
