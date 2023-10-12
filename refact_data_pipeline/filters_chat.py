import logging
import traceback
from typing import Dict, List

import numpy as np

from code_contrast.format_2023q2 import format, packing
from code_contrast.format_2023q2.el_msg import MsgElement
from code_contrast.format_2023q2.element import Format2023q2
from refact_data_pipeline import DatasetOpts
from refact_encoding.encoding import RefactEncoding


class Chat2023Q2:
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts
    ):
        self.inner_filter = inner_filter
        self.n_ctx = dataopts.get("n_ctx", 2048)
        self.no_format_prob = dataopts.get("chat_no_format_prob", 0.0)
        self.debug = bool(dataopts.get("debug", 0))
        self.tkr_stochastic_tokens = bool(dataopts.get("tkr_stochastic_tokens", 0.0))
        self.enc: RefactEncoding = dataopts.encoding
        self.fmt: Format2023q2 = format.format_2023q2_escape(self.enc)
        self.random = np.random.RandomState(dataopts.get("seed", 42))

    def _pack_format(self, plan: List[MsgElement], odm: Dict, stats: Dict):
        try:
            pack = packing.Packer(self.fmt)
            for p in plan:
                pack.add_to_plan(p)
            pack.pack_context(
                start_from_plan_n=0,
                mask_from_plan_n=0,
                limit_ctx_n=self.n_ctx,
                limit_aux_n=0,
                add_eot=True,
                for_training=True
            )
        except Exception:
            msg = "{\n"
            for key, val in odm.items():
                msg += f"    {repr(key)}: {repr(val)},\n"
            msg += "}"
            logging.error(msg)
            logging.error(traceback.format_exc())
            stats["chatskip_failed"] += 1
            return None
        first = [1] + [0] * (len(pack.r) - 1)
        assert len(pack.r) == len(first)
        assert len(pack.r) == len(pack.m)
        emit = {
            "tokens": pack.r,
            "mask": pack.m,
            "first": first,
            "stats": {**odm["stats"], **stats}
        }
        return emit

    def _pack_plain(self, plan: List[MsgElement], odm: Dict, stats: Dict):
        system_dict = [
            'instructions: {message}', 'context: {message}', 'instructions:\n{message}', 'context:\n{message}',
            'system: {message}', 'system:\n{message}', 'question: {message}', 'question:\n{message}',
            'Q: {message}', 'Q:\n{message}',
        ]
        user_dict = [
            'user: {message}', 'user:\n{message}', 'question: {message}', 'question:\n{message}',
            'Q: {message}', 'Q:\n{message}', 'instruction: {message}', 'instruction:\n{message}'
        ]
        assistant_dict = [
            'output: {message}', 'output:\n{message}', 'answer: {message}', 'answer:\n{message}',
            'A: {message}', 'A:\n{message}', 'reply: {message}', 'reply:\n{message}',
            'response: {message}', 'response:\n{message}', 'assistant: {message}', 'assistant:\n{message}'
        ]
        text = ""
        for p in plan:
            if p.msg_role == "SYSTEM":
                text += f"{self.random.choice(system_dict).format(message=p.msg_text)}\n"
            elif p.msg_role == "USER":
                text += f"{self.random.choice(user_dict).format(message=p.msg_text)}\n"
            elif p.msg_role == "ASSISTANT":
                text += f"{self.random.choice(assistant_dict).format(message=p.msg_text)}"

        if self.debug:
            print(f'Chat2023Q2:\n{text}\n\n')

        tokens, _ = self.enc.encode_stochastic(text, [], 0.01 * self.tkr_stochastic_tokens)
        tokens += [self.enc.EOT]
        emit = {
            "tokens": tokens,
            "mask": [1] * len(tokens),
            "first": [1] + [0] * (len(tokens) - 1),
            "stats": stats
        }
        return emit

    def __iter__(self):
        stats: Dict[str, int] = {
            "chatskip_failed": 0,
        }
        for odm in self.inner_filter:
            assert len(odm['chat']) > 0
            plan = []
            for item in odm['chat']:
                if len(item['instruction']) > 0:
                    plan.append(MsgElement("SYSTEM", item['instruction']))
                if len(item['input']) > 0:
                    plan.append(MsgElement("USER", item['input']))
                plan.append(MsgElement("ASSISTANT", item['output']))

            if self.random.random() > self.no_format_prob:
                emit = self._pack_format(plan, odm, stats)
            else:
                emit = self._pack_plain(plan, odm, stats)

            if emit is None:
                continue
            yield emit
