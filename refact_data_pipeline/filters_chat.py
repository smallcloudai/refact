import traceback
import traces
from typing import Dict

from code_contrast.format_2023q2 import format, packing
from code_contrast.format_2023q2.el_msg import MsgElement
from code_contrast.format_2023q2.element import Format2023q2
from code_contrast.format_2023q2.from_orig_dest_message import from_odm_dict
from refact_data_pipeline import DatasetOpts
from refact_encoding.encoding import RefactEncoding


class Chat2023Q2FromODM:
    def __init__(self,
                 inner_filter,
                 dataopts: DatasetOpts):
        self.inner_filter = inner_filter
        self.n_ctx = dataopts.get("n_ctx", 2048)
        self.enc: RefactEncoding = dataopts.encoding
        self.fmt: Format2023q2 = format.format_2023q2_escape(self.enc)

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
            except Exception as e:
                msg = "{\n"
                for key, val in odm.items():
                    msg += f"    {repr(key)}: {repr(val)},\n"
                msg += "}"
                traces.log(msg)
                traces.log(traceback.format_exc())
                stats["chatskip_failed"] += 1
                continue
            first = [1] + [0] * (len(pack.r) - 1)
            assert len(pack.r) == len(first)
            assert len(pack.r) == len(pack.m)
            emit = {
                "tokens": pack.r,
                "mask": pack.m,
                "first": first,
                "stats": {**odm["stats"], **stats}
            }
            yield emit
