import functools
import math

import torch as th
from torch import nn


def _get_slopes(attn_heads: int, dev: str) -> th.Tensor:
    """
    ## Get head-specific slope $m$ for each head
    * `n_heads` is the number of heads in the attention layer $n$
    The slope for first head is
    $$\frac{1}{2^{\frac{8}{n}}} = 2^{-\frac{8}{n}}$$
    The slopes for the rest of the heads are in a geometric series with a ratio same as above.
    For instance when the number of heads is $8$ the slopes are
    $$\frac{1}{2^1}, \frac{1}{2^2}, \dots, \frac{1}{2^8}$$
    """

    # Get the closest power of 2 to `n_heads`.
    # If `n_heads` is not a power of 2, then we first calculate slopes to the closest (smaller) power of 2,
    # and then add the remaining slopes.
    n = 2 ** math.floor(math.log2(attn_heads))
    # $2^{-\frac{8}{n}}$
    m_0 = 2.0 ** (-8.0 / n)
    # $2^{-1\frac{8}{n}}, 2^{-2 \frac{8}{n}}, 2^{-3 \frac{8}{n}}, \dots$
    m = th.pow(m_0, th.arange(1, 1 + n, device=dev))

    # If `n_heads` is not a power of 2, then we add the remaining slopes.
    # We calculate the remaining slopes for $n * 2$ (avoiding slopes added previously).
    # And pick the slopes upto `n_heads`.
    if n < attn_heads:
        # $2^{-\frac{8}{2n}}$
        m_hat_0 = 2.0 ** (-4.0 / n)
        # $2^{-1\frac{8}{2n}}, 2^{-3 \frac{8}{2n}}, 2^{-5 \frac{8}{2n}}, \dots$
        # Note that we take steps by $2$ to avoid slopes added previously.
        m_hat = th.pow(m_hat_0, th.arange(1, 1 + 2 * (attn_heads - n), 2, device=dev))
        # Concatenate the slopes with the remaining slopes.
        m = th.cat([m, m_hat])

    return m


@functools.lru_cache(maxsize=1)
def _get_alibi_biases(
        B: int,
        T: int,
        attn_heads: int,
        dev: str,
        dtype,
        causal: bool = True) -> th.Tensor:
    """
    ## Calculate the attention biases matrix
    * `n_heads` is the number of heads in the attention layer
    * `mask` is the attention mask of shape `[seq_len_q, seq_len_k]`
    This returns a matrix of shape `[seq_len_q, seq_len_k, n_heads, ]` with ALiBi attention biases.
    """

    # Get slopes $m$ for each head
    if causal:
        mask = (th.triu(th.ones((T, T), device=dev)) == 1).transpose(0, 1)
    else:
        mask = th.ones((T, T), device=dev, dtype=th.bool)

    m = _get_slopes(attn_heads, dev)

    # Calculate distances $[0, 1, \dots, N]$
    # Here we calculate the distances using the mask.
    #
    # Since it's causal mask we can just use $[0, 1, \dots, N]$ too.
    # `distance = torch.arange(mask.shape[1], dtype=torch.long, device=mask.device)[None, :]`
    distance = mask.cumsum(dim=-1)

    # Multiply them pair-wise to get the AliBi bias matrix
    biases = distance[:, :, None] * m[None, None, :]
    biases = biases.permute(2, 0, 1)[None, :, :T, :T]
    biases = biases.repeat(B, 1, 1, 1)
    return biases.to(dtype).contiguous()


class ALiBiBias(nn.Module):
    def __init__(
            self, config,
            causal: bool = True
    ):
        super().__init__()
        self.attn_heads = config.attn_heads
        self.max_T = config.T
        self.causal = causal
        self.biases = None

    @th.no_grad()
    def forward(self, B: int, t: int, T: int, device, dtype) -> th.Tensor:
        if self.biases is None or self.biases.shape[0] < B or \
                self.biases.shape[-1] < T or self.biases.shape[-2] < T:
            self.biases = _get_alibi_biases(
                B, self.max_T, self.attn_heads, device, dtype, self.causal
            )

        return self.biases[:B, :, -t:, :T]
