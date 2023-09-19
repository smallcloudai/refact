import random
from typing import Any, Dict, List

import binpacking
import numpy as np
import psutil
from scipy.special import softmax

from refact_data_pipeline import DatasetOpts

ItemT = Dict[str, Any]


class Packer:
    """
    Pack several tokenized records along time axis.
    Stat dict comes from last inner record.
    """

    def __init__(self,
                 inner_filter,
                 dataopts: DatasetOpts,
                 force16: bool = False,
                 force_pack_complete: bool = False,
                 force_pack1: bool = False,
                 keys: List[str] = ["tokens", "mask", "first"]
                 ):
        self.inner_filter = inner_filter
        self.enc = dataopts.encoding
        self.pack_at_most: int = dataopts.get("pack_at_most", 6)
        if force_pack1:
            self.pack_at_most = 1
        self.pack_complete: int = dataopts.get("pack_complete", 0) == 1 or force_pack_complete
        self.pack_pad0: int = dataopts.get("pack_pad0", 1) == 1
        self.n_ctx: int = dataopts.get("n_ctx", 2048)
        self.force16 = force16
        self.keys = keys

    def __iter__(self):
        accum = {k: list() for k in self.keys}
        stats: Dict[str, int] = {
            "packed_in": 0,
            "packed_out": 0,
            "packed_skip5tokens": 0,
        }
        last_rec_stats = dict()

        def dict_to_emit():
            nonlocal accum
            stats["packed_out"] += 1
            stats["pusher_resmem"] = psutil.Process().memory_info().rss / 1e9
            last_rec_stats.update(stats)
            accum_cut = {k: v[:self.n_ctx] for k, v in accum.items()}
            emit = {
                "stats": {**last_rec_stats, **stats},
                **accum_cut,
            }
            if self.pack_pad0:
                for k in self.keys:
                    if k == "tokens":
                        emit[k].extend([self.enc.DIAMOND] * (self.n_ctx - len(emit[k])))
                    else:
                        emit[k].extend([0] * (self.n_ctx - len(emit[k])))
            accum = {k: accum[k][self.n_ctx:] for k in self.keys}
            return emit

        packed_n = 0
        for rec in self.inner_filter:
            if sum(rec["mask"]) < 5:
                stats["packed_skip5tokens"] += 1
                continue
            last_rec_stats = rec["stats"]
            stats["packed_in"] += 1
            existing_len = len(accum[self.keys[0]])
            if self.pack_complete:
                predict_len = existing_len + len(rec["tokens"])
                if existing_len > 0 and (
                        predict_len >= self.n_ctx or packed_n >= self.pack_at_most
                ):
                    yield dict_to_emit()
                    for a in accum.values():
                        a.clear()
                    packed_n = 0
            for k in self.keys:
                accum[k].extend(rec[k])
            while self.force16 and len(accum[self.keys[0]]) & 15:
                padlen = 16 - (len(accum[self.keys[0]]) & 15)
                for k in self.keys:
                    if k == "tokens":
                        accum[k].extend([self.enc.DIAMOND] * padlen)
                    else:
                        accum[k].extend([0] * padlen)
            packed_n += 1
            if not self.pack_complete:
                while len(accum[self.keys[0]]) >= self.n_ctx:
                    yield dict_to_emit()
                    packed_n = 1
            len0 = len(accum[self.keys[0]])
            assert all(len0 == len(accum[k]) for k in self.keys[1:])
        if len(accum[self.keys[0]]):
            yield dict_to_emit()


class SinglePacker:
    """
    Pack several tokenized records along time axis.
    Stat dict comes from last inner record.
    """

    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts,
            keys: List[str] = ["tokens", "first"]
    ):
        self.inner_filter = inner_filter
        self.enc = dataopts.encoding
        self.n_ctx: int = dataopts.get("n_ctx", 2048)
        self.keys = keys

    def __iter__(self):
        for rec in self.inner_filter:
            output = dict(stats=rec["stats"])
            for k in self.keys:
                if len(rec[k]) < self.n_ctx:
                    rec[k] += [self.enc.DIAMOND] * (self.n_ctx - len(rec[k]))
                output[k] = rec[k][:self.n_ctx]
            output["mask"] = [t != self.enc.DIAMOND for t in output['tokens']]
            yield output


class DensePacker:
    """
    Pack several tokenized records along the time axis.
    Stat dict comes from last inner record.
    """

    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts,
    ):
        self.inner_filter_iter = iter(inner_filter)
        self.enc = dataopts.encoding
        self.n_ctx: int = dataopts['n_ctx']
        self.pack_single: bool = dataopts.get('pack_single', 0) == 1
        self.pack_complete: bool = dataopts.get('pack_complete', 1) == 1
        self.drop_less_than_t: int = dataopts.get('pack_drop_less_than_t', 6)
        self.buffer_size: int = dataopts.get('pack_buffer_size', 256)
        self.keys = dataopts.get('packer_keys', 'tokens;mask;first').split(';')
        self.max_packing_rounds = 8
        self.do_nothing_keys = ['stats']
        assert len(self.keys) > 0
        self.buffer = []
        self.stats = dict(
            packed_in=0,
            packed_out=0,
            packed_small_dropped=0,
            last_paddings_perc=0.0
        )

    def __make_padded_item(self, length: int) -> ItemT:
        padded_item = dict()
        for k in self.keys:
            if k == 'tokens':
                padded_item[k] = [self.enc.DIAMOND for _ in range(length)]
            elif k in {'mask', 'first'}:
                padded_item[k] = [0 for _ in range(length)]
            else:
                assert f'Unknown key={k} to process'
        return padded_item

    def __item_len(self, item: ItemT) -> int:
        return len(item[self.keys[0]])

    def __items_len(self, items):
        return sum(self.__item_len(i) for i in items)

    def __fill_buffer(self):
        while True:
            item = next(self.inner_filter_iter, None)
            if item is None:
                break
            if self.__item_len(item) <= self.drop_less_than_t:
                self.stats['packed_small_dropped'] += 1
                continue
            if len(self.buffer) < self.buffer_size:
                self.buffer.append(item)
            else:
                break

    def __add_to_acc(
            self,
            items_acc: List[ItemT],
            items_to_add: List[ItemT]
    ) -> List[ItemT]:
        left_overs = []
        for item in items_to_add:
            item_to_add, left_over_item = dict(), dict()
            length_to_add = self.n_ctx - self.__items_len(items_acc)
            for key in self.keys:
                item_to_add[key] = item[key][:length_to_add]
                left_over_item[key] = item[key][length_to_add:]
            for key in self.do_nothing_keys:
                if key not in item:
                    continue
                item_to_add[key] = item[key]
                left_over_item[key] = item[key]
            items_acc.append(item_to_add)
            if not self.pack_complete and self.__item_len(left_over_item) > self.drop_less_than_t:
                left_overs.append(left_over_item)
            elif not self.pack_complete and self.__item_len(left_over_item) <= self.drop_less_than_t:
                self.stats['packed_small_dropped'] += 1
        return left_overs

    def __find_best_for_budget(self, budget: int, force_random_get: bool = False) -> List[ItemT]:
        def _pop_item_by_length(length: int) -> ItemT:
            idx = next((idx for idx, item in enumerate(self.buffer)
                        if self.__item_len(item) == length), None)
            assert idx is not None, f'No item with length={length}'
            return self.buffer.pop(idx)

        if len(self.buffer) == 0 or budget == 0:
            return []

        if force_random_get or not self.pack_complete:
            item = self.buffer.pop(random.randint(0, len(self.buffer) - 1))
            return [item]
        else:
            lengths = [self.__item_len(i) for i in self.buffer]
            lengths = [l for l in lengths if l <= budget]
            if len(lengths) == 0:
                return []
            # we can up-weight `old` items later
            bins = binpacking.to_constant_volume(lengths, budget)
            if len(bins) == 0:
                return []

            # prioritize items with larger lengths
            p = softmax(np.exp(np.array([sum(b) for b in bins]) / budget * 2))
            bin = bins[np.random.choice(list(range(len(bins))), p=p)]
            items = [_pop_item_by_length(l) for l in bin]
            return items

    def __merge_items(
            self,
            items_acc: List[ItemT],
            random_order: bool
    ) -> ItemT:
        assert len(items_acc) > 0

        if random_order:
            np.random.shuffle(items_acc)
        last_item = items_acc[-1]
        if self.__items_len(items_acc) < self.n_ctx:
            items_acc.append(self.__make_padded_item(self.n_ctx - self.__items_len(items_acc)))

        output_item = dict([(k, []) for k in self.keys])
        # taking the last item for other useful keys
        output_item.update(dict([(k, last_item[k]) for k in self.do_nothing_keys if k in last_item]))
        if 'stats' in output_item:
            output_item['stats'].update(self.stats)
        else:
            output_item['stats'] = self.stats

        for item in items_acc:
            for k in self.keys:
                output_item[k].extend(item[k])

        return output_item

    def __iter__(self):
        def _pack_iteration(acc, force_random_get=False):
            items = self.__find_best_for_budget(
                budget=self.n_ctx - self.__items_len(acc),
                force_random_get=force_random_get
            )
            if len(items) > 0:
                self.stats['packed_in'] += len(items)
                leftovers = self.__add_to_acc(acc, items)
                self.buffer.extend(leftovers)
            return len(items)

        def _merge_acc(acc):
            assert len(acc) > 0
            self.stats['packed_out'] += 1
            output_item = self.__merge_items(acc, random_order=True)
            if 'tokens' in output_item:
                self.stats['last_paddings_perc'] = \
                    (np.array(output_item['tokens']) == self.enc.DIAMOND).sum() / self.n_ctx
            return output_item

        while True:
            self.__fill_buffer()
            if len(self.buffer) == 0:
                break

            items_acc = []
            _pack_iteration(acc=items_acc, force_random_get=True)
            if self.pack_single:
                yield _merge_acc(acc=items_acc)
                continue

            for _ in range(self.max_packing_rounds):
                packed = _pack_iteration(acc=items_acc)
                if packed == 0:
                    break

            yield _merge_acc(acc=items_acc)
