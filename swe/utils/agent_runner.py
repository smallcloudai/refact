import shutil
import subprocess
import traceback

from uuid import uuid4
from async_timeout import timeout
from datasets import load_dataset
from pathlib import Path
from typing import Dict, Any

from swe import SWE_WORKDIR
from refact.lsp_runner import LSPServerRunner


__all__ = [
    "get_swe_bench_lite_instance",
    "AgentRunner",
]


def get_swe_bench_lite_instance(instance_id: str):
    swebench = {
        row["instance_id"]: {
            "repo": row["repo"],
            "base_commit": row["base_commit"],
            "problem_statement": row["problem_statement"],
            "patch": row["patch"],
        }
        for row in load_dataset('princeton-nlp/SWE-bench_Lite', split='test')
    }
    assert instance_id in swebench
    return swebench[instance_id]


class RepoContext:

    def __init__(self, repo: str, base_commit: str, workdir: Path):
        self._workdir = workdir
        self._repo_name = repo
        self._base_commit = base_commit
        self._context_repo_path = workdir / str(uuid4())
        self._workdir.mkdir(exist_ok=True, parents=True)

    async def __aenter__(self):
        repo_path = self._workdir / self._repo_name.split("/")[-1]
        if not repo_path.exists():
            # subprocess.call(["git", "clone", f"git@github.com:{self._repo_name}.git"], cwd=self._workdir)
            subprocess.call(["git", "clone", f"https://github.com/{self._repo_name}"], cwd=self._workdir)
        assert repo_path.exists()
        assert not self._context_repo_path.exists()
        subprocess.call(["cp", "-r", str(repo_path), str(self._context_repo_path)])
        subprocess.call(["git", "clean", "-fd"], cwd=str(self._context_repo_path))
        subprocess.call(["git", "reset", "--hard", self._base_commit], cwd=str(self._context_repo_path))
        subprocess.call(["git", "--no-pager", "log", "-1"], cwd=str(self._context_repo_path))
        return self._context_repo_path

    async def __aexit__(self, exc_type, exc, tb):
        if self._context_repo_path.exists():
            shutil.rmtree(str(self._context_repo_path))


class AgentRunner:
    def __init__(self, timeout, use_ast: bool, use_vecdb: bool):
        self._timeout = timeout
        self._use_ast = use_ast
        self._use_vecdb = use_vecdb
        self._repos_workdir = SWE_WORKDIR / "repos"

    async def _steps(self, base_url: str, repo_path: Path, **kwargs) -> Dict[str, Any]:
        raise NotImplementedError()

    async def run(self, repo_name: str, base_commit: str, **kwargs):
        try:
            async with RepoContext(repo_name, base_commit, self._repos_workdir) as repo_path:
                async with LSPServerRunner(repo_path=str(repo_path), use_ast=self._use_ast, use_vecdb=self._use_vecdb) as runner:
                    async with timeout(self._timeout):
                        return await self._steps(base_url=runner.base_url, repo_path=repo_path, **kwargs)
        except Exception as e:
            raise e
            return {
                "error": f"run: {str(e) or traceback.format_exc()}",
            }
