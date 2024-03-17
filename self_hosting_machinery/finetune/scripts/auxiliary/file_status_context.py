from dataclasses import dataclass
from pathlib import Path
from typing import Optional, Any, Dict, List

from refact_utils.scripts import env
from refact_utils.finetune.utils import get_file_digest
from self_hosting_machinery.finetune.scripts.auxiliary.finetune_filter_status_tracker import FinetuneFilterStatusTracker
from self_hosting_machinery.finetune.utils import traces


@dataclass
class FileStatus:
    path: Path
    info: Dict[str, Any]
    is_train: bool
    status: Optional[str] = None
    reason: Optional[str] = None

    def hash(self) -> str:
        assert self.path.exists(), f"File {self.path} doesn't exist, try to rescan your files"
        return get_file_digest(self.path)


class FilesStatusContext:
    def __init__(
            self,
            pname: str,
            train_files: List[Dict[str, Any]],
            test_files: List[Dict[str, Any]],
            status_tracker: FinetuneFilterStatusTracker
    ):
        self.pname = pname
        self.file_statuses: Dict[str, FileStatus] = {
            info["path"]: FileStatus(path=Path(env.PP_DIR_UNPACKED(pname)) / info["path"], info=info, is_train=True)
            for info in train_files
        }
        self.file_statuses.update({
            info["path"]: FileStatus(path=Path(env.PP_DIR_UNPACKED(pname)) / info["path"], info=info, is_train=False)
            for info in test_files
        })
        self.log_files_accepted_ftf = Path(env.PP_LOG_FILES_ACCEPTED_FTF(pname))
        self.log_files_rejected_ftf = Path(env.PP_LOG_FILES_REJECTED_FTF(pname))
        with self.log_files_accepted_ftf.open('w') as f:
            f.write("")
        with self.log_files_rejected_ftf.open('w') as f:
            f.write("")
        self._global_stats = status_tracker
        self._check_prerequisites()

    def _check_prerequisites(self):
        train_hashes_dict = {
            f.hash(): f for f in self.file_statuses.values() if f.is_train
        }
        train_hashes = set(train_hashes_dict.keys())
        test_hashes = set(
            f.hash() for f in self.file_statuses.values() if not f.is_train
        )
        inters = train_hashes.intersection(test_hashes)
        if len(inters) > 0:
            paths = [train_hashes_dict[h].path for h in inters]
            raise RuntimeError(f"Provided similar files in train and test set: {paths}")

    def _change_file_status(self, file: Dict[str, Any], status: str, reason: str, log_file: Path):
        assert file["path"] in self.file_statuses
        file_status = self.file_statuses[file["path"]]
        file_status.status = status
        file_status.reason = reason
        try:
            with open(log_file, "a", encoding="utf-8") as f:
                f.write(f"{reason} {file['path']}\n")
        except Exception as e:
            traces.log(f"Couldn't fill the log file {log_file}: {e}")
            raise e

    def accept_file(self, file: Dict[str, Any], reason: str):
        self._change_file_status(file, "accepted", reason, self.log_files_accepted_ftf)
        self._global_stats.set_accepted_num(self.accepted_files_num)

    def reject_file(self, file: Dict[str, Any], reason: str):
        traces.log(f"REJECTED FILTER {file['path']:<100} {reason}")
        self._change_file_status(file, "rejected", reason, self.log_files_rejected_ftf)
        self._global_stats.set_rejected_num(self.rejected_files_num)

    def no_status_train_files(self) -> List[Dict[str, Any]]:
        """
        :return: List of files that are train and not have status
        """
        return [
            f.info for f in self.file_statuses.values()
            if f.status is None and f.is_train
        ]

    def no_status_test_files(self) -> List[Dict[str, Any]]:
        """
        :return: List of files that are test and not have status
        """
        return [
            f.info for f in self.file_statuses.values()
            if f.status is None and not f.is_train
        ]

    def accepted_train_files(self) -> List[Dict[str, Any]]:
        """
        :return: List of train files with accepted status
        """
        return [
            f.info for f in self.file_statuses.values()
            if f.status == "accepted" and f.is_train
        ]

    @property
    def accepted_files_num(self) -> int:
        return sum(s.status == "accepted" for s in self.file_statuses.values())

    @property
    def rejected_files_num(self) -> int:
        return sum(s.status == "rejected" for s in self.file_statuses.values())
