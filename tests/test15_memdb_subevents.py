import asyncio
import json
from asyncio import sleep
from dataclasses import dataclass
from typing import Dict, List

import aiohttp
import termcolor

BASE_URL = "http://127.0.0.1:8001"
quit_flag = False


@dataclass
class Memory:
    memid: str
    m_type: str
    m_goal: str
    m_project: str
    m_payload: str
    m_origin: str
    mstat_correct: float
    mstat_relevant: float
    mstat_times_used: int


@dataclass
class MemdbSubEvent:
    pubevent_id: str
    pubevent_action: str
    pubevent_memid: str
    pubevent_json: str

    def print(self):
        colors = {
            "INSERT": "green",
            "UPDATE": "yellow",
            "DELETE": "red",
        }
        print(termcolor.colored(f"{self.pubevent_id} [{self.pubevent_action}]: {self.pubevent_json}",
                                colors[self.pubevent_action]))


@dataclass
class VecdbStatusEvent:
    files_unprocessed: int
    files_total: int
    requests_made_since_start: int
    vectors_made_since_start: int
    db_size: int
    db_cache_size: int
    state: str
    queue_additions: bool
    vecdb_max_files_hit: bool
    vecdb_errors: str

    def print(self):
        print(self)


tasks: Dict[str, asyncio.Task] = {}


async def memdb_sub(session):
    def receive_sub_event(line):
        decoded_line = line.decode('utf-8').strip()
        if decoded_line.startswith("data: "):
            decoded_line = decoded_line[6:].strip()
        j = json.loads(decoded_line)
        try:
            event = MemdbSubEvent(**j)
            event.print()
        except Exception as e:
            try:
                status = VecdbStatusEvent(**j)
                status.print()
            except Exception as e:
                print(termcolor.colored(f"Failed to decode event: {e}", "red"))

    global quit_flag
    try:
        while True:
            try:
                async with session.post(f"{BASE_URL}/v1/mem-sub") as response:
                    if response.status != 200:
                        print(termcolor.colored(f"Failed to connect to SSE. Status code: {response.status}", "red"))
                        await sleep(1)
                        continue
                    else:
                        async for line in response.content:
                            if not line.strip():
                                continue
                            receive_sub_event(line)
            except Exception as e:
                print(termcolor.colored(f"Failed to connect to refact-lsp: {e}", "red"))
                await sleep(1)
                continue
    except Exception as e:
        print(termcolor.colored(f"Exception occurred: {str(e)}", "red"))
    finally:
        quit_flag = True


async def main():
    session = aiohttp.ClientSession()
    tasks = [
        asyncio.create_task(memdb_sub(session)),
    ]
    try:
        await asyncio.gather(*tasks)
    finally:
        await session.close()


if __name__ == "__main__":
    asyncio.run(main())
