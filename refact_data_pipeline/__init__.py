import importlib
from typing import Union

from refact_data_pipeline.datadef import DatasetDef, DatasetMix, DatasetOpts, DatasetDumpedDef


def find_dataset_def(dsname: str) -> Union[DatasetDef, DatasetMix]:
    submod_name, name = dsname.split(":")
    mod = importlib.import_module("data_pipeline." + submod_name)
    assert name in mod.__dict__, "dataset '%s' was not found in '%s'" % (name, submod_name)
    f = getattr(mod, name)
    assert callable(f)
    d = f()
    assert isinstance(d, (DatasetDef, DatasetMix, DatasetDumpedDef))
    return d
