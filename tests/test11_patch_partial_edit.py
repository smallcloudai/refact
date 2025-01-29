import asyncio
import difflib
import json
import os
import tempfile
from pathlib import Path
from typing import Tuple

import numpy as np
import requests
import textdistance
from datasets import load_dataset
from refact.lsp_runner import LSPServerRunner
from termcolor import colored
from tqdm import tqdm

try:
    REFACT_API_KEY = os.environ['REFACT_API_KEY']
except KeyError:
    print("Please set REFACT_API_KEY env variable")
    exit(1)


def unified_diff(a, b):
    def color_diff(diff):
        for line in diff:
            if line.startswith('+'):
                yield colored(line, 'green')
            elif line.startswith('-'):
                yield colored(line, 'red')
            elif line.startswith('^'):
                yield colored(line, 'blue')
            else:
                yield line

    a_lines = a.splitlines(keepends=True)
    b_lines = b.splitlines(keepends=True)
    diff = difflib.unified_diff(a_lines, b_lines, lineterm='')
    return ''.join(color_diff(diff))


def get_changed_block(a: str, b: str, padding: int = 2):
    a_lines = a.splitlines(keepends=True)
    b_lines = b.splitlines(keepends=True)
    diff = difflib.unified_diff(a_lines, b_lines, lineterm='')

    changed_lines = set()
    for line in diff:
        if line.startswith('@@'):
            # Extract the line numbers from the diff header
            header_parts = line.split()
            new_file_range = header_parts[2]
            start_line = int(new_file_range.split(',')[0][1:])
            num_lines = int(new_file_range.split(',')[1])
            for i in range(start_line, start_line + num_lines):
                changed_lines.add(i)

    min_line, max_line = min(changed_lines) - padding, max(changed_lines) + padding
    min_line = max(0, min_line)
    max_line = min(len(a_lines), max_line)
    return "".join(b_lines[min_line:max_line])


def patch_request(messages, ticket_ids, base_url: str):
    payload = {
        "messages": messages,
        "ticket_ids": ticket_ids,
    }
    resp = requests.post(
        f"{base_url}/patch-single-file-from-ticket",
        data=json.dumps(payload),
        timeout=240
    )
    assert resp.status_code == 200, resp.text
    return resp.json()


def make_messages(ticket_text: str):
    return [
        {"role": "assistant", "content": ticket_text}
    ]


def make_refact_lsp(workspace_path: str):
    return LSPServerRunner(
        refact_lsp_command=[
            '/home/svakhreev/projects/refact-lsp/target/debug/refact-lsp',
            '--address-url',
            'Refact',
            '--api-key',
            f'{REFACT_API_KEY}',
            '--ast',
            f'--workspace-folder={workspace_path}',
        ],
        wait_for_ast_vecdb=True,
        refact_lsp_log="./log.txt",
        verbose=False
    )


def materialize_file_temporary(text: str, suffix) -> Tuple[tempfile.TemporaryDirectory, str]:
    temp_dir = tempfile.TemporaryDirectory()
    temp_file = tempfile.NamedTemporaryFile(delete=False, dir=temp_dir.name, suffix=suffix)
    with open(temp_file.name, 'w') as f:
        f.write(text)
    return temp_dir, temp_file.name


async def entrypoint(ds):
    incorrect_distances = []
    correct, complete_errors, incorrect_diffs = 0, 0, 0
    for repo in tqdm(list(ds['train'])[:100]):
        try:
            print(f'Processing {repo["filename"]}')
            if len(incorrect_distances) > 0:
                print(f"Mean text distance for invalid results: {np.mean(incorrect_distances):.2f}:\n")
                print(f"Correct: {correct}, incorrect: {incorrect_diffs}, complete errors: {complete_errors}")
            text_before, text_after = repo['file_before'], repo['file_after']
            text_after_changed_only = get_changed_block(text_before, text_after)
            project_path, filename = materialize_file_temporary(text_before, suffix=Path(repo['filename']).suffix)
            refact_lsp = make_refact_lsp(project_path.name)
            await refact_lsp.start()
            messages = make_messages(f"📍SECTION_EDIT 001 {filename}\n```\n{text_after_changed_only}\n```")
            resp = patch_request(messages, ["001"], base_url=refact_lsp.base_url())
            await refact_lsp.stop()
            text_after = text_after.strip()
            resp["results"][0]["file_text"] = resp["results"][0]["file_text"].strip()
            if text_after != resp["results"][0]["file_text"]:
                diff = unified_diff(resp["results"][0]["file_text"], text_after)
                incorrect_distances.append(1.0 - textdistance.jaro_winkler(text_after, resp["results"][0]["file_text"]))
                incorrect_diffs += 1
                print(f'There is some difference:\n{diff}\n')
            else:
                correct += 1
                print("Result is correct\n")
        except Exception as e:
            complete_errors += 1
            print(f"Error: {e}, skip to the next example")
            continue
    print(f"Mean text distance for invalid results: {np.mean(incorrect_distances)}")
    print(f"Correct: {correct}, incorrect: {incorrect_diffs}, complete errors: {complete_errors}")


if __name__ in '__main__':
    print(f"Downloading diffs_generation_test...")
    ds = load_dataset("smallcloudai/diffs_generation_test", cache_dir="./.diffs_generation_test")
    asyncio.run(entrypoint(ds))
