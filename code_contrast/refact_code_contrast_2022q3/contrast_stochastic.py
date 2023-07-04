import copy
import random
import difflib
import termcolor
import numpy as np
from typing import Optional

from encoding_wrapper.refact_encoding import RefactEncoding


text_a = """#hmm
class XYZ(object):
    def __init__(self, aa, bb, cc):
        self.x = x
        self.y = y
"""

text_b = """#import math
class XYZ(object):
    def __init__(self, x, y, z):
        self.x = x
        self.y = y
        self.z = z
"""


def apply_ops(a, b, ops):
    scratch = copy.deepcopy(a)
    a2scratch = list(range(len(scratch) + 1))   # Initially 1:1, differs after edits
    for op, i1, i2, j1, j2 in ops:
        if op == "equal":
            continue
        cursor = a2scratch.index(i1)
        scratch[cursor:cursor + (i2-i1)] = b[j1:j2]
        a2scratch[cursor:cursor + (i2-i1)] = [-1] * (j2-j1)
    return scratch


def test_ops(a_tokens, b_tokens, ops):
    assert(apply_ops(a_tokens, b_tokens, ops) == b_tokens)
    ops.reverse()
    assert(apply_ops(a_tokens, b_tokens, ops) == b_tokens)
    for _ in range(50):
        random.shuffle(ops)
        assert(apply_ops(a_tokens, b_tokens, ops) == b_tokens)


def ops_remove_short_equals(ops, upto):
    if upto == 0:
        return ops
    result = copy.deepcopy(ops)
    while 1:
        n = 1
        did_anything = False
        while n < len(result) - 1:
            lop, li1, li2, lj1, lj2 = result[n-1]
            mop, mi1, mi2, mj1, mj2 = result[n]
            rop, ri1, ri2, rj1, rj2 = result[n+1]
            if mop == "equal" and mi2 - mi1 <= upto:
                assert lop != "equal"
                assert rop != "equal"
                assert upto > 0
                result[n-1:n+2] = [("joined", li1, ri2, lj1, rj2)]
                did_anything = True
                break
            n += 1
        if not did_anything:
            break
    return result


def ops_stochastic_expand(
    ops,
    *,
    left_prob,
    right_prob,
    disable_insert,
    exact_cx_lines0=-1,
    exact_cx_lines1=-1,
    np_random: Optional[np.random.RandomState] = None,
):
    def poisson():
        if np_random is None:
            return np.random.poisson(lam=2)
        else:
            return np_random.poisson(lam=2)
    result = copy.deepcopy(ops)
    # move left boundary
    for n in range(1, len(result)-1):
        lop, li1, li2, lj1, lj2 = result[n-1]
        mop, mi1, mi2, mj1, mj2 = result[n]
        if lop == "equal" and mop != "equal" and random.random() < left_prob:
            assert li2 == mi1
            if exact_cx_lines0 >= 0:
                move = exact_cx_lines0
            else:
                move = poisson()
                move = min(li2 - li1 - 1, move)
            if move < li2 - li1 and move > 0:
                result[n-1] = (lop, li1, li2 - move, lj1, lj2 - move)
                result[n] = (mop, mi1 - move, mi2, mj1 - move, mj2)
    # move right boundary
    for n in range(0, len(result)-1):
        mop, mi1, mi2, mj1, mj2 = result[n]
        rop, ri1, ri2, rj1, rj2 = result[n+1]
        # if mop != "equal" and rop == "equal" and (random.random() < right_prob or (mi1==mi2 and disable_insert)):
        if mop != "equal" and rop == "equal" and random.random() < right_prob:
            assert ri1 == mi2
            if exact_cx_lines1 >= 0:
                move = exact_cx_lines1
            else:
                # if disable_insert, add at least one line => insert becomes replace
                move = poisson()
                if disable_insert:
                    move = max(1, move)
                else:
                    move = min(ri2 - ri1 - 1, move)
            if move < ri2 - ri1 and move > 0:
                result[n] = (mop, mi1, mi2 + move, mj1, mj2 + move)
                result[n+1] = (rop, ri1 + move, ri2, rj1 + move, rj2)
    return result


def test_stochastic(remove_short_equals=False, stochastic_replace_more=True):
    enc = RefactEncoding("openai_programming_v2")
    def dec(x):
        return enc.decode(x).replace("\n", "\\n")
    a_tokens = enc.encode(text_a)
    b_tokens = enc.encode(text_b)
    patch = difflib.SequenceMatcher(None, a_tokens, b_tokens, autojunk=False)
    ops = list(patch.get_opcodes())
    upto = random.randint(1, 10)
    if remove_short_equals:
        ops = ops_remove_short_equals(ops, upto=upto)
    if stochastic_replace_more:
        ops = ops_stochastic_expand(ops, upto=upto)
    prev_i1 = 0
    print("-"*100)
    for op, i1, i2, j1, j2 in ops:
        if op == "equal":
            print(termcolor.colored(op, "magenta"), str(a_tokens[i1:i2]))
            print(dec(a_tokens[i1:i2]))
        elif op in ["replace", "insert", "delete", "joined"]:
            print(op, termcolor.colored(str(a_tokens[i1:i2]), "red"), termcolor.colored(str(b_tokens[j1:j2]), "green"))
            if i2 > i1:
                print(termcolor.colored(dec(a_tokens[i1:i2]), "red"))
            if j2 > j1:
                print(termcolor.colored(dec(b_tokens[j1:j2]), "green"))
        else:
            print("unknown op:", op)
        assert i1 >= prev_i1
        prev_i1 = i1
    try:
        test_ops(a_tokens, b_tokens, ops)
    except AssertionError:
        print("remove_short_equals", remove_short_equals, "stochastic_replace_more", stochastic_replace_more, "upto", upto)
        raise


if __name__=="__main__":
    for _ in range(10):
        test_stochastic(remove_short_equals=True, stochastic_replace_more=False)
    for _ in range(10):
        test_stochastic(remove_short_equals=False, stochastic_replace_more=True)
