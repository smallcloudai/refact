import whatthepatch

from pathlib import Path


def patched_file(patch: str) -> str:
    files = list(whatthepatch.parse_patch(patch))
    assert len(files) == 1
    header = files[0].header
    assert header.old_path[len("a/"):] == header.new_path[len("b/"):]
    return header.old_path[len("a/"):]


def filename_mentioned(filename: str, text: str) -> str:
    if filename in text:
        return "fully"
    elif Path(filename).name in text:
        return "partially"
    return "no"
