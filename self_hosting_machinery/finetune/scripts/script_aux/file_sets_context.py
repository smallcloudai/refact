import random
from pathlib import Path
from typing import List, Dict, Any

import jsonlines

from self_hosting_machinery.finetune.utils import traces
from self_hosting_machinery.scripts import env

__all__ = ['FileSetsContext']

DIR_UNPACKED = Path(env.DIR_UNPACKED)


class FileSetsContext:
    UNFILTERED_TRAIN_FILENAME = DIR_UNPACKED / "train_set.jsonl"
    FILTERED_TRAIN_FILENAME = DIR_UNPACKED / "train_set_filtered.jsonl"
    UNFILTERED_TEST_FILENAME = DIR_UNPACKED / "test_set.jsonl"
    FILTERED_TEST_FILENAME = DIR_UNPACKED / "test_set_filtered.jsonl"

    TRAIN_FILES_MIN_NUMBER_WITH_TEST_SET = 4
    TRAIN_FILES_MIN_NUMBER_WITHOUT_TEST_SET = 7
    TEST_FILES_COUNT_WARNING = 64

    def __init__(self, autoselect_test_files_num: int):
        self._check_prerequisites()
        self.autoselect_test_files_num = autoselect_test_files_num
        self.train_files: List[Dict[str, Any]] = list(jsonlines.open(self.UNFILTERED_TRAIN_FILENAME))
        self.test_files: List[Dict[str, Any]] = list(jsonlines.open(self.UNFILTERED_TEST_FILENAME))

    def _check_prerequisites(self):
        if not self.UNFILTERED_TRAIN_FILENAME.exists():
            raise RuntimeError("No train files have been provided")

        train_files = list(jsonlines.open(self.UNFILTERED_TRAIN_FILENAME))
        test_files = list(jsonlines.open(self.UNFILTERED_TEST_FILENAME))
        train_min_number = (
            self.TRAIN_FILES_MIN_NUMBER_WITH_TEST_SET if len(test_files) > 0 else
            self.TRAIN_FILES_MIN_NUMBER_WITHOUT_TEST_SET
        )
        if len(train_files) < train_min_number:
            raise RuntimeError(f"Provided train set is too small ({len(train_files)} files)\n"
                               f"It should contain at least {train_min_number} files")

        if len(test_files) > self.TEST_FILES_COUNT_WARNING:
            traces.log(f"Manually selected test set contains {len(test_files)} files. "
                       f"It could heavily slow down the training process on the next stage")

    def is_up_to_date(self) -> bool:
        unfiltered_train, filtered_train = (
            self.UNFILTERED_TRAIN_FILENAME, self.FILTERED_TRAIN_FILENAME
        )
        unfiltered_test, filtered_test = (
            self.UNFILTERED_TRAIN_FILENAME, self.FILTERED_TRAIN_FILENAME
        )
        how_to_filter = Path(env.CONFIG_HOW_TO_FILTER)
        how_to_filetypes = Path(env.CONFIG_HOW_TO_FILETYPES)

        try:
            has_updates = [
                unfiltered_train.lstat().st_mtime > filtered_train.lstat().st_mtime,
                unfiltered_test.lstat().st_mtime > filtered_test.lstat().st_mtime,
            ]
            if how_to_filter.exists():
                has_updates.append(how_to_filter.lstat().st_mtime > filtered_train.lstat().st_mtime)
            if how_to_filetypes.exists():
                has_updates.append(how_to_filetypes.lstat().st_mtime > filtered_train.lstat().st_mtime)
        except OSError:
            return False
        return not any(has_updates)

    def dump_filtered(
            self,
            files: List[Dict[str, Any]]
    ):
        def _dump(files, filename):
            with jsonlines.open(filename, "w") as f:
                for file in files:
                    f.write(file)

        if len(self.test_files) == 0:
            test_files_count = min(self.autoselect_test_files_num, len(self.train_files) // 2)
            if test_files_count == 0:
                raise RuntimeError(
                    "It is too little files to choose a test set from. "
                    "It's strongly recommended to choose a test set manually to be able to prevent overfitting"
                )
            else:
                random.shuffle(files)
                test_files = files[:test_files_count]
                train_files = files[test_files_count:]
        else:
            train_files = files
            test_files = self.test_files

        _dump(train_files, self.FILTERED_TRAIN_FILENAME)
        _dump(test_files, self.FILTERED_TEST_FILENAME)
        traces.log("-" * 40 + "TEST SET" + "-" * 40)
        for file in test_files:
            traces.log(file["path"])
