import json
from pathlib import Path
from typing import List, Set, Tuple


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


class DatasetDumpedDef:
    def __init__(
            self,
            path: str,
            to_apply: Set[str],
            suffixes: Tuple[str, ...] = ('.h5', '.hdf5')
    ):
        assert not path.startswith('gs://'), "DatasetDumpedDef doesn't support cloud-based paths " \
                                             "because of random access to files"
        # Those paths are not cloud, just for names compatibility
        self.cloud_path = path
        self.cloud_files = [p for p in sorted(Path(path).iterdir(), key=lambda p: p.name)
                            if p.suffix in suffixes]
        self.to_apply = to_apply

    def __repr__(self):
        return "dataset definition %s with %i files and filters %s" % (
            self.cloud_path, len(self.cloud_path), str(self.to_apply))


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
                    try:
                        self.opts[k] = int(v)
                    except ValueError:
                        self.opts[k] = v
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

