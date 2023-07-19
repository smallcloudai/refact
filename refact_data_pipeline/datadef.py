import json
from typing import List, Set


class DatasetDef:
    def __init__(self,
        cloud_path: str,
        cloud_files: List[str],
        to_apply: Set[str]
    ):
        self.cloud_path = cloud_path
        self.cloud_files = cloud_files
        self.to_apply = to_apply

    def __repr__(self):
        return "dataset definition %s with %i files and filters %s" % (
            self.cloud_path, len(self.cloud_files), str(self.to_apply))


class DatasetMix:
    def __init__(self,
        dataset_defs: List[DatasetDef],
        proportions: List[float] = [],
    ):
        self.dataset_defs = dataset_defs
        self.proportions = proportions


class DatasetOpts:
    def __init__(self, s):
        self.opts = dict()
        if len(s):
            for t in s.split(","):
                k, v = t.split("=")
                if "." in v:
                    self.opts[k] = float(v)
                else:
                    self.opts[k] = int(v)
        self.used = set()
        self.encoding = None

    def set_encoding(self, enc):
        self.encoding = enc

    def __getitem__(self, k):
        self.used.add(k)
        return self.opts[k]

    def get(self, k, default):
        self.used.add(k)
        return self.opts[k] if k in self.opts else default

    def __contains__(self, k):
        return k in self.opts

    def assert_all_used(self):
        unused = set(self.opts.keys()) - self.used
        assert not unused, "DatasetOpts has unused data processing options %s" % str(unused)

    def __repr__(self):
        return json.dumps(self.opts)

