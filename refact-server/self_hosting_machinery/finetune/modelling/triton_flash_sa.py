import functools
import math
from typing import Optional

import torch as th
import triton
import triton.language as tl
from einops import einops

from self_hosting_machinery.finetune.modelling.utils import get_base_model
from self_hosting_machinery.finetune.utils import traces


@functools.lru_cache(maxsize=1)
def _get_alibi_slopes(attn_heads: int, dev: str) -> th.Tensor:
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

    return m.to(th.float32)


@triton.heuristics(
    {
        "EVEN_HEADDIM": lambda args: args["HEAD_DIM"] == args["BLOCK_HEADDIM"],
    }
)
@triton.jit
def _fwd_kernel(
        Q, K, V, alibi_slope,
        O, L,
        softmax_scale,
        stride_qb, stride_qm, stride_qh,
        stride_kb, stride_kn, stride_kh,
        stride_vb, stride_vn, stride_vh,
        stride_ob, stride_om, stride_oh,
        seq_len: tl.constexpr,
        FUSED_ALIBI: tl.constexpr,
        HAS_REACH: tl.constexpr, REACH: tl.constexpr,
        N_HEADS: tl.constexpr, KV_N_HEADS: tl.constexpr,
        HEAD_DIM: tl.constexpr, BLOCK_HEADDIM: tl.constexpr,
        IS_CAUSAL: tl.constexpr, BLOCK_M: tl.constexpr, BLOCK_N: tl.constexpr,
        EVEN_HEADDIM: tl.constexpr
):
    start_m = tl.program_id(0)
    off_hb = tl.program_id(1)
    off_b = off_hb // N_HEADS
    off_h = off_hb % N_HEADS
    # initialize offsets
    offs_m = start_m * BLOCK_M + tl.arange(0, BLOCK_M)
    offs_n = tl.arange(0, BLOCK_N)
    offs_d = tl.arange(0, BLOCK_HEADDIM)
    if FUSED_ALIBI:
        alibi_slope = tl.load(alibi_slope + off_h)
    # Initialize pointers to Q, K, V
    q_ptrs = Q + off_b * stride_qb + off_h * stride_qh + (offs_m[:, None] * stride_qm) + offs_d[None, :]
    if KV_N_HEADS > 1:
        k_ptrs = K + off_b * stride_kb + off_h * stride_kh + (offs_n[None, :] * stride_kn) + offs_d[:, None]
        v_ptrs = V + off_b * stride_vb + off_h * stride_vh + (offs_n[:, None] * stride_vn) + offs_d[None, :]
    else:
        k_ptrs = K + (off_b * stride_kb) + (offs_n[None, :] * stride_kn) + offs_d[:, None]
        v_ptrs = V + (off_b * stride_vb) + (offs_n[:, None] * stride_vn) + offs_d[None, :]
    # initialize pointer to m and l
    lse_i = tl.zeros([BLOCK_M], dtype=tl.float32) - float('inf')
    m_i = tl.zeros([BLOCK_M], dtype=tl.float32) - float('inf')
    acc = tl.zeros([BLOCK_M, BLOCK_HEADDIM], dtype=tl.float32)
    # load q: it will stay in SRAM throughout
    if EVEN_HEADDIM:
        q = tl.load(q_ptrs)
    else:
        q = tl.load(q_ptrs, mask=offs_d[None, :] < HEAD_DIM, other=0.0)

    if not IS_CAUSAL:
        end_n = seq_len
        begin_m = 0
    else:
        end_n = tl.minimum((start_m + 1) * BLOCK_M, seq_len)
        if HAS_REACH:
            begin_m = tl.maximum(0, end_n - ((REACH + 1) * BLOCK_M))
        else:
            begin_m = 0
    for start_n in range(begin_m, end_n, BLOCK_N):
        # -- compute qk ----
        if EVEN_HEADDIM:
            k = tl.load(k_ptrs)
        else:
            k = tl.load(k_ptrs, mask=offs_d[:, None] < HEAD_DIM, other=0.0)
        qk = tl.zeros([BLOCK_M, BLOCK_N], dtype=tl.float32)
        qk += tl.dot(q, k)
        qk *= softmax_scale
        if FUSED_ALIBI:
            bias = alibi_slope * (start_n + tl.arange(0, BLOCK_N)[None, :])
            qk += bias

        if IS_CAUSAL:
            qk = tl.where(offs_m[:, None] >= (start_n + offs_n[None, :]), qk, float('-inf'))
        m_ij = tl.maximum(tl.max(qk, 1), lse_i)
        p = tl.exp(qk - m_ij[:, None])
        l_ij = tl.sum(p, 1)

        # scale acc_o
        acc_scale = tl.exp(m_i - m_ij)
        acc = acc * acc_scale[:, None]
        if EVEN_HEADDIM:
            v = tl.load(v_ptrs)
        else:
            v = tl.load(v_ptrs, mask=offs_d[None, :] < HEAD_DIM, other=0.0)
        p = p.to(v.dtype)
        acc += tl.dot(p, v)  # registration for dialect for op: builtin.unrealized_conversion_cast

        # update statistics
        m_i = m_ij
        l_i_new = tl.exp(lse_i - m_ij) + l_ij
        lse_i = m_ij + tl.log(l_i_new)

        # update pointers
        k_ptrs += BLOCK_N * stride_kn
        v_ptrs += BLOCK_N * stride_vn

    acc_scale = tl.exp(m_i - lse_i)
    acc = acc * acc_scale[:, None]
    # rematerialize offsets to save registers
    start_m = tl.program_id(0)
    offs_m = start_m * BLOCK_M + tl.arange(0, BLOCK_M)
    # write back l
    l_ptrs = L + off_hb * seq_len + offs_m
    tl.store(l_ptrs, lse_i)
    # initialize pointers to output
    offs_d = tl.arange(0, BLOCK_HEADDIM)
    out_ptrs = O + off_b * stride_ob + off_h * stride_oh + offs_m[:, None] * stride_om + offs_d[None, :]
    if EVEN_HEADDIM:
        tl.store(out_ptrs, acc)
    else:
        tl.store(out_ptrs, acc, mask=offs_d[None, :] < HEAD_DIM)


@triton.heuristics(
    {
        "EVEN_HEADDIM": lambda args: args["HEAD_DIM"] == args["BLOCK_HEADDIM"],
    }
)
@triton.jit
def _bwd_preprocess_do_o_dot(
        Out, DO, Delta,
        stride_ob, stride_om, stride_oh,
        stride_dob, stride_dom, stride_doh,
        seq_len: tl.constexpr,
        N_HEADS: tl.constexpr, HEAD_DIM: tl.constexpr,
        BLOCK_M: tl.constexpr, BLOCK_HEADDIM: tl.constexpr,
        EVEN_HEADDIM: tl.constexpr
):
    start_m = tl.program_id(0)
    off_hb = tl.program_id(1)
    off_b = off_hb // N_HEADS
    off_h = off_hb % N_HEADS
    # initialize offsets
    offs_m = start_m * BLOCK_M + tl.arange(0, BLOCK_M)
    offs_d = tl.arange(0, BLOCK_HEADDIM)
    # load
    if EVEN_HEADDIM:
        o = tl.load(Out + off_b * stride_ob + off_h * stride_oh + offs_m[:, None] * stride_om + offs_d[None, :]).to(
            tl.float32)
        do = tl.load(DO + off_b * stride_dob + off_h * stride_doh + offs_m[:, None] * stride_dom + offs_d[None, :]).to(
            tl.float32)
    else:
        o = tl.load(Out + off_b * stride_ob + off_h * stride_oh + offs_m[:, None] * stride_om + offs_d[None, :],
                    mask=offs_d[None, :] < HEAD_DIM, other=0.0).to(tl.float32)
        do = tl.load(DO + off_b * stride_dob + off_h * stride_doh + offs_m[:, None] * stride_dom + offs_d[None, :],
                     mask=offs_d[None, :] < HEAD_DIM, other=0.0).to(tl.float32)
    delta = tl.sum(o * do, axis=1)
    # write-back
    tl.store(Delta + off_hb * seq_len + offs_m, delta)


@triton.jit
def _bwd_kernel_one_col_block(
        start_n,
        Q, K, V, alibi_slope,
        DO, DQ, DK, DV,
        L, D,
        softmax_scale,
        stride_qm, stride_kn, stride_vn,
        stride_dom, stride_dqm, stride_dkn, stride_dvn,
        seqlen: tl.constexpr,
        FUSED_ALIBI: tl.constexpr,
        HAS_REACH: tl.constexpr, REACH: tl.constexpr,
        HEAD_DIM: tl.constexpr, BLOCK_HEADDIM: tl.constexpr,
        IS_CAUSAL: tl.constexpr, ATOMIC_ADD: tl.constexpr,
        BLOCK_M: tl.constexpr, BLOCK_N: tl.constexpr,
        EVEN_HEADDIM: tl.constexpr
):
    # We need to make sure begin_m is a multiple of BLOCK_M (not BLOCK_N)
    begin_m = 0 if not IS_CAUSAL else ((start_n * BLOCK_N) // BLOCK_M) * BLOCK_M
    # initialize row/col offsets
    offs_qm = begin_m + tl.arange(0, BLOCK_M)
    offs_n = start_n * BLOCK_N + tl.arange(0, BLOCK_N)
    offs_m = tl.arange(0, BLOCK_M)
    offs_d = tl.arange(0, BLOCK_HEADDIM)
    # initialize pointers to value-like data
    q_ptrs = Q + (offs_qm[:, None] * stride_qm + offs_d[None, :])
    k_ptrs = K + (offs_n[:, None] * stride_kn + offs_d[None, :])
    v_ptrs = V + (offs_n[:, None] * stride_vn + offs_d[None, :])
    do_ptrs = DO + (offs_qm[:, None] * stride_dom + offs_d[None, :])
    dq_ptrs = DQ + (offs_qm[:, None] * stride_dqm + offs_d[None, :])
    # initialize dv and dk
    dv = tl.zeros([BLOCK_N, BLOCK_HEADDIM], dtype=tl.float32)
    dk = tl.zeros([BLOCK_N, BLOCK_HEADDIM], dtype=tl.float32)
    # k and v stay in SRAM throughout
    if EVEN_HEADDIM:
        k = tl.load(k_ptrs)
        v = tl.load(v_ptrs)
    else:
        k = tl.load(k_ptrs, mask=offs_d[None, :] < HEAD_DIM, other=0.0)
        v = tl.load(v_ptrs, mask=offs_d[None, :] < HEAD_DIM, other=0.0)
    # loop over rows
    num_block_m = tl.cdiv(seqlen, BLOCK_M)
    if FUSED_ALIBI:
        if IS_CAUSAL:
            b = alibi_slope * (begin_m + tl.arange(0, BLOCK_N)[None, :])
        else:
            b = alibi_slope * ((start_n * BLOCK_M) + tl.arange(0, BLOCK_N)[None, :])
    if HAS_REACH:
        end_m = tl.minimum(num_block_m * BLOCK_M, begin_m + REACH * BLOCK_M)
    else:
        end_m = num_block_m * BLOCK_M
    for start_m in range(begin_m, end_m, BLOCK_M):
        offs_m_curr = start_m + offs_m
        # -- compute qk ----
        if EVEN_HEADDIM:
            q = tl.load(q_ptrs)
        else:
            q = tl.load(q_ptrs, mask=offs_d[None, :] < HEAD_DIM, other=0.0)
        qk = tl.dot(q, tl.trans(k))
        qk *= softmax_scale
        if FUSED_ALIBI:
            qk += b
        if IS_CAUSAL:
            qk = tl.where(offs_m_curr[:, None] >= (offs_n[None, :]), qk, float('-inf'))
        # There seems to be a race condition when headdim=48/96, and dq, dk, dv are wrong.
        # Also wrong for headdim=64.
        # tl.debug_barrier()
        lse_i = tl.load(L + offs_m_curr)
        p = tl.exp(qk - lse_i[:, None])
        # compute dv
        # [2022-10-30] TD: A Triton bug: if EVEN_M=True and EVEN_HEADDIM=False, if we call
        # do = tl.load(do_ptrs, mask=offs_d[None, :] < headdim, other=0.0), we get wrong outputs
        # in the case of headdim=48/96, seqlen_q & seqlen_k >= 512. If headdim=40 or seqlen < 512,
        # the output is correct.
        if EVEN_HEADDIM:
            do = tl.load(do_ptrs)
        else:
            do = tl.load(do_ptrs, mask=offs_d[None, :] < HEAD_DIM, other=0.0)
        dv += tl.dot(tl.trans(p.to(Q.dtype.element_ty)), do)
        # compute dp = dot(v, do)
        # There seems to be a race condition when headdim=48/96, and dq, dk are wrong.
        # Also wrong for headdim=128, seqlen=(108, 256), and ATOMIC_ADD=True
        # Also wrong for headdim=64, seqlen=(1023, 1024), and ATOMIC_ADD=False
        # tl.debug_barrier()
        dp = tl.dot(do, tl.trans(v))
        # There's a race condition for headdim=48
        # tl.debug_barrier()
        # compute ds = p * (dp - delta[:, None])
        # Putting the subtraction after the dp matmul (instead of before) is slightly faster
        Di = tl.load(D + offs_m_curr)
        # Converting ds to q.dtype here reduces register pressure and makes it much faster
        # for BLOCK_HEADDIM=128
        ds = (p * (dp - Di[:, None]) * softmax_scale).to(q.dtype)
        # compute dk = dot(ds.T, q)
        dk += tl.dot(tl.trans(ds), q)
        # compute dq
        # tl.debug_barrier()
        if not ATOMIC_ADD:
            if EVEN_HEADDIM:
                dq = tl.load(dq_ptrs)
                dq += tl.dot(ds, k)
                tl.store(dq_ptrs, dq)
            else:
                dq = tl.load(dq_ptrs, mask=offs_d[None, :] < HEAD_DIM, other=0.0)
                dq += tl.dot(ds, k)
                tl.store(dq_ptrs, dq, mask=offs_d[None, :] < HEAD_DIM)
        else:
            if EVEN_HEADDIM:
                dq = tl.dot(ds, k)
                tl.atomic_add(dq_ptrs, dq)
            else:
                dq = tl.dot(ds, k)
                tl.atomic_add(dq_ptrs, dq, mask=offs_d[None, :] < HEAD_DIM)
        # increment pointers
        dq_ptrs += BLOCK_M * stride_dqm
        q_ptrs += BLOCK_M * stride_qm
        do_ptrs += BLOCK_M * stride_dom
    # write-back
    dv_ptrs = DV + (offs_n[:, None] * stride_dvn + offs_d[None, :])
    dk_ptrs = DK + (offs_n[:, None] * stride_dkn + offs_d[None, :])
    if EVEN_HEADDIM:
        tl.store(dv_ptrs, dv)
        tl.store(dk_ptrs, dk)
    else:
        tl.store(dv_ptrs, dv, mask=offs_d[None, :] < HEAD_DIM)
        tl.store(dk_ptrs, dk, mask=offs_d[None, :] < HEAD_DIM)


@triton.heuristics(
    {
        "EVEN_HEADDIM": lambda args: args["HEAD_DIM"] == args["BLOCK_HEADDIM"],
    }
)
@triton.jit
def _bwd_kernel(
        Q, K, V, alibi_slope,
        DO, DQ, DK, DV,
        L, D,
        softmax_scale,
        stride_qb, stride_qm, stride_qh,
        stride_kb, stride_kn, stride_kh,
        stride_vb, stride_vn, stride_vh,
        stride_dob, stride_dom, stride_doh,
        stride_dqb, stride_dqm, stride_dqh,
        stride_dkb, stride_dkn, stride_dkh,
        stride_dvb, stride_dvn, stride_dvh,
        seq_len: tl.constexpr,
        FUSED_ALIBI: tl.constexpr,
        HAS_REACH: tl.constexpr, REACH: tl.constexpr,
        N_HEADS: tl.constexpr,
        HEAD_DIM: tl.constexpr, BLOCK_HEADDIM: tl.constexpr,
        SEQUENCE_PARALLEL: tl.constexpr, IS_CAUSAL: tl.constexpr,
        BLOCK_M: tl.constexpr, BLOCK_N: tl.constexpr,
        EVEN_HEADDIM: tl.constexpr,
):
    off_hb = tl.program_id(1)
    off_b = off_hb // N_HEADS
    off_h = off_hb % N_HEADS
    # offset pointers for batch/head
    Q += off_b * stride_qb + off_h * stride_qh
    K += off_b * stride_kb + off_h * stride_kh
    V += off_b * stride_vb + off_h * stride_vh
    if FUSED_ALIBI:
        alibi_slope = tl.load(alibi_slope + off_h)
    DO += off_b * stride_dob + off_h * stride_doh
    DQ += off_b * stride_dqb + off_h * stride_dqh
    DK += off_b * stride_dkb + off_h * stride_dkh
    DV += off_b * stride_dvb + off_h * stride_dvh
    # pointer to row-wise quantities in value-like data
    D += off_hb * seq_len
    L += off_hb * seq_len
    if not SEQUENCE_PARALLEL:
        num_block_n = tl.cdiv(seq_len, BLOCK_N)
        for start_n in range(0, num_block_n):
            _bwd_kernel_one_col_block(
                start_n,
                Q, K, V, alibi_slope,
                DO, DQ, DK, DV,
                L, D,
                softmax_scale,
                stride_qm, stride_kn, stride_vn,
                stride_dom, stride_dqm, stride_dkn, stride_dvn,
                seq_len,
                FUSED_ALIBI=FUSED_ALIBI,
                HAS_REACH=HAS_REACH, REACH=REACH,
                HEAD_DIM=HEAD_DIM, BLOCK_HEADDIM=BLOCK_HEADDIM,
                IS_CAUSAL=IS_CAUSAL, ATOMIC_ADD=False,
                BLOCK_M=BLOCK_M, BLOCK_N=BLOCK_N,
                EVEN_HEADDIM=EVEN_HEADDIM
            )
    else:
        start_n = tl.program_id(0)
        _bwd_kernel_one_col_block(
            start_n,
            Q, K, V, alibi_slope,
            DO, DQ, DK, DV,
            L, D,
            softmax_scale,
            stride_qm, stride_kn, stride_vn,
            stride_dom, stride_dqm, stride_dkn, stride_dvn,
            seq_len,
            FUSED_ALIBI=FUSED_ALIBI,
            HAS_REACH=HAS_REACH, REACH=REACH,
            HEAD_DIM=HEAD_DIM, BLOCK_HEADDIM=BLOCK_HEADDIM,
            IS_CAUSAL=IS_CAUSAL, ATOMIC_ADD=False,
            BLOCK_M=BLOCK_M, BLOCK_N=BLOCK_N,
            EVEN_HEADDIM=EVEN_HEADDIM
        )


def _flash_attn_forward(
        q: th.Tensor,
        k: th.Tensor,
        v: th.Tensor,
        causal: bool,
        softmax_scale: float,
        fused_alibi_bias: bool,
        reach: Optional[int]
):
    # shape constraints
    batch, seqlen, nheads, d = q.shape
    _, seqlen_k, kv_nheads, _ = k.shape
    assert k.shape == (batch, seqlen_k, kv_nheads, d)
    assert v.shape == (batch, seqlen_k, kv_nheads, d)
    assert d <= 128, 'FlashAttention only support head dimensions up to 128'
    assert q.dtype == k.dtype == v.dtype, 'All tensors must have the same type'
    assert q.dtype in [th.float16, th.bfloat16], 'Only support fp16 and bf16'
    assert q.is_cuda and k.is_cuda and v.is_cuda
    assert not (
            reach is not None and not causal), 'FlashAttention does not support reach and non causal at the same time'

    l = th.empty((batch, nheads, seqlen), device=q.device, dtype=th.float32)
    o = th.empty_like(q)
    alibi_slope = _get_alibi_slopes(attn_heads=nheads, dev=q.device) if fused_alibi_bias else None

    BLOCK_HEADDIM = max(triton.next_power_of_2(d), 16)
    BLOCK = 128
    num_warps = 4 if d <= 64 else 8
    _fwd_kernel[(triton.cdiv(seqlen, BLOCK), batch * nheads)](
        q, k, v, alibi_slope,
        o, l,
        softmax_scale,
        q.stride(0), q.stride(1), q.stride(2),
        k.stride(0), k.stride(1), k.stride(2),
        v.stride(0), v.stride(1), v.stride(2),
        o.stride(0), o.stride(1), o.stride(2),
        seqlen,
        FUSED_ALIBI=fused_alibi_bias,
        HAS_REACH=reach is not None, REACH=reach,
        N_HEADS=nheads, KV_N_HEADS=kv_nheads,
        HEAD_DIM=d, BLOCK_HEADDIM=BLOCK_HEADDIM,
        IS_CAUSAL=causal, BLOCK_M=BLOCK, BLOCK_N=BLOCK,
        num_warps=num_warps, num_stages=1
    )
    return o, l


def _flash_attn_backward(
        do: th.Tensor,
        q: th.Tensor, k: th.Tensor, v: th.Tensor,
        o: th.Tensor, lse: th.Tensor,
        dq: th.Tensor, dk: th.Tensor, dv: th.Tensor,
        causal: bool,
        softmax_scale: float,
        fused_alibi_bias: bool,
        reach: Optional[int]
):
    if do.stride(-1) != 1:
        do = do.contiguous()
    batch, seqlen, nheads, d = q.shape
    _, seqlen_k, kv_nheads, _ = k.shape
    assert d <= 128
    assert lse.shape == (batch, nheads, seqlen)
    assert q.stride(-1) == k.stride(-1) == v.stride(-1) == o.stride(-1) == 1
    assert dq.stride(-1) == dk.stride(-1) == dv.stride(-1) == 1
    delta = th.empty_like(lse)
    alibi_slope = _get_alibi_slopes(attn_heads=nheads, dev=q.device) if fused_alibi_bias else None

    BLOCK_HEADDIM = max(triton.next_power_of_2(d), 16)
    grid = lambda META: (triton.cdiv(seqlen, META["BLOCK_M"]), batch * nheads)
    _bwd_preprocess_do_o_dot[grid](
        o, do, delta,
        o.stride(0), o.stride(1), o.stride(2),
        do.stride(0), do.stride(1), do.stride(2),
        seqlen,
        N_HEADS=nheads, HEAD_DIM=d,
        BLOCK_M=128, BLOCK_HEADDIM=BLOCK_HEADDIM
    )
    grid = lambda META: (
        triton.cdiv(seqlen_k, META["BLOCK_N"]) if META["SEQUENCE_PARALLEL"] else 1,
        batch * nheads
    )
    if kv_nheads == 1:
        k = k.expand((batch, seqlen_k, nheads, d)).contiguous()
        v = v.expand((batch, seqlen_k, nheads, d)).contiguous()
    _bwd_kernel[grid](
        q, k, v, alibi_slope,
        do, dq, dk, dv,
        lse, delta,
        softmax_scale,
        q.stride(0), q.stride(1), q.stride(2),
        k.stride(0), k.stride(1), k.stride(2),
        v.stride(0), v.stride(1), v.stride(2),
        do.stride(0), do.stride(1), do.stride(2),
        dq.stride(0), dq.stride(1), dq.stride(2),
        dk.stride(0), dk.stride(1), dk.stride(2),
        dv.stride(0), dv.stride(1), dv.stride(2),
        seq_len=seqlen,
        FUSED_ALIBI=fused_alibi_bias,
        HAS_REACH=reach is not None, REACH=reach,
        N_HEADS=nheads,
        HEAD_DIM=d, BLOCK_HEADDIM=BLOCK_HEADDIM,
        SEQUENCE_PARALLEL=True, IS_CAUSAL=causal,
        BLOCK_M=128, BLOCK_N=128,
        num_warps=8, num_stages=2
    )


class FlashAttnFunc(th.autograd.Function):
    @staticmethod
    def forward(
            ctx,
            q: th.Tensor,
            k: th.Tensor,
            v: th.Tensor,
            softmax_scale: float,
            causal: bool = False,
            fused_alibi_bias: bool = False,
            reach: Optional[int] = None
    ):
        # Make sure that the last dimension is contiguous
        q, k, v = [x if x.stride(-1) == 1 else x.contiguous() for x in [q, k, v]]
        o, lse = _flash_attn_forward(
            q, k, v, causal=causal, softmax_scale=softmax_scale,
            fused_alibi_bias=fused_alibi_bias, reach=reach
        )
        ctx.save_for_backward(q, k, v, o, lse)
        ctx.softmax_scale = softmax_scale
        ctx.causal = causal
        ctx.fused_alibi_bias = fused_alibi_bias
        ctx.reach = reach
        return o

    @staticmethod
    def backward(ctx, do):
        q, k, v, o, lse = ctx.saved_tensors
        # Triton's autotune causes the Tensor._version to change, and so Pytorch autograd
        # does a memcpy. To avoid this we run in inference_mode, which doesn't track the version.
        with th.inference_mode():
            dq = th.zeros_like(q)
            if q.shape[2] > 1 and k.shape[2] == 1:
                dk = th.empty_like(q)
                dv = th.empty_like(q)
            else:
                dk = th.empty_like(k)
                dv = th.empty_like(v)
            _flash_attn_backward(
                do, q, k, v, o, lse, dq, dk, dv,
                causal=ctx.causal, softmax_scale=ctx.softmax_scale,
                fused_alibi_bias=ctx.fused_alibi_bias, reach=ctx.reach
            )
        return dq, dk, dv, None, None, None, None


flash_attn_func = FlashAttnFunc.apply


def apply_flash_mha_to_refact_model(model):
    def _forward(
            self,
            x: th.Tensor,
            layer_past: Optional[th.Tensor] = None,
            attention_mask: Optional[th.Tensor] = None,
            alibi: Optional[th.Tensor] = None,
            use_cache: Optional[bool] = False,
            output_attentions: Optional[bool] = False,
            *args, **kwargs
    ):
        q = einops.rearrange(self.q(x), "b t (h d) -> b t h d", h=self.num_heads)
        kv = einops.rearrange(self.kv(x), "b t (h d) -> b t h d", h=2)
        k, v = kv.chunk(2, dim=2)

        attn_output = flash_attn_func(
            q, k, v, self.scale_factor, True, True
        )
        attn_output = einops.rearrange(attn_output, "b t h d -> b t (h d)")

        attn_output = self.c_proj(attn_output)
        return attn_output, None

    if th.cuda.get_device_capability() < (8, 0):
        model.force_low_gpu_mem_mode = True
        traces.log("Triton flash attention is not supported on gpus with cuda capability < 8")
        return

    traces.log("Applying triton flash attention to the model")
    model = get_base_model(model)
    for block in model.transformer.h:
        block.attn.forward = _forward.__get__(block.attn, type(block.attn))
