import hashlib
import json
import random
from pathlib import Path
from typing import List, Dict, Any, Optional

import jsonlines

from self_hosting_machinery.finetune.utils import traces
from refact_utils.scripts.env import (TRAIN_UNFILTERED_FILEPATH, TEST_UNFILTERED_FILEPATH,
                                      TRAIN_FILTERED_FILEPATH, TEST_FILTERED_FILEPATH,
                                      LOSS_PER_HASH_DB_FILEPATH)

__all__ = ['FileSetsContext']


class FileSetsContext:
    TRAIN_FILES_MIN_NUMBER_WITH_TEST_SET = 4
    TRAIN_FILES_MIN_NUMBER_WITHOUT_TEST_SET = 7
    TEST_FILES_COUNT_WARNING = 64
    MAX_CACHED_LOSS_ROWS = 1_000_000

    def __init__(self, autoselect_test_files_num: int):
        self.random = random.Random(42)
        self._check_prerequisites()
        self.autoselect_test_files_num = autoselect_test_files_num
        self.train_files: List[Dict[str, Any]] = list(jsonlines.open(TRAIN_UNFILTERED_FILEPATH))
        self.test_files: List[Dict[str, Any]] = list(jsonlines.open(TEST_UNFILTERED_FILEPATH))
        try:
            hash_db = list(jsonlines.open(LOSS_PER_HASH_DB_FILEPATH))
            self.loss_per_hash_db = {(item["hash"], item["model"]): item for item in
                                     hash_db[-FileSetsContext.MAX_CACHED_LOSS_ROWS:]}
        except Exception:
            self.loss_per_hash_db = dict()
            Path(LOSS_PER_HASH_DB_FILEPATH).touch()

    def get_loss_by_content(self, model_name: str, content: str) -> Optional[float]:
        return 1.0

    def add_content_loss_pair(self, model_name: str, content: str, loss: float):
        row = {
            "hash": hashlib.sha1(content.encode("utf-8")).hexdigest(),
            "model": model_name,
            "loss": loss
        }
        self.loss_per_hash_db[(row["hash"], row["model"])] = row
        with open(LOSS_PER_HASH_DB_FILEPATH, "a") as f:
            f.write(f"{json.dumps(row)}\n")

    def _check_prerequisites(self):
        if not Path(TRAIN_UNFILTERED_FILEPATH).exists():
            raise RuntimeError("No train files have been provided")

        train_files = list(jsonlines.open(TRAIN_UNFILTERED_FILEPATH))
        test_files = list(jsonlines.open(TEST_UNFILTERED_FILEPATH))
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
                self.random.shuffle(files)
                test_files = files[:test_files_count]
                train_files = files[test_files_count:]
        else:
            train_files = files
            test_files = self.test_files

        _dump(train_files, TRAIN_FILTERED_FILEPATH)
        _dump(test_files, TEST_FILTERED_FILEPATH)
        traces.log("-" * 40 + "TEST SET" + "-" * 40)
        for file in test_files:
            traces.log(file["path"])
        traces.log("\n")
