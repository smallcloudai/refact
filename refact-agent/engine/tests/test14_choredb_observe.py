import aiohttp
import asyncio
import termcolor
import json
import argparse
import time
from pydantic import BaseModel, ConfigDict
from typing import Optional, Dict, List
from pydantic import BaseModel, PrivateAttr
from refact import chat_client


BASE_URL = "http://127.0.0.1:8001"
SLEEP = 3
quit_flag = False

class CMessage(BaseModel):
    cmessage_belongs_to_cthread_id: str
    cmessage_alt: int
    cmessage_num: int
    cmessage_prev_alt: int
    cmessage_usage_model: str
    cmessage_usage_prompt: int
    cmessage_usage_completion: int
    cmessage_json: str

def cmessage_key(cmessage_belongs_to_cthread_id, cmessage_alt, cmessage_num):
    return "%s:%d:%d" % (cmessage_belongs_to_cthread_id, cmessage_alt, cmessage_num)

class CThread(BaseModel):
    cthread_id: str
    cthread_belongs_to_chore_event_id: Optional[str]
    cthread_title: str
    cthread_toolset: str
    cthread_model: str
    cthread_error: str
    cthread_anything_new: bool
    cthread_created_ts: float
    cthread_updated_ts: float
    cthread_archived_ts: float

class ChoreEvent(BaseModel):
    chore_event_id: str
    chore_event_belongs_to_chore_id: str
    chore_event_summary: str
    chore_event_ts: float
    chore_event_link: str
    chore_event_cthread_id: Optional[str]

class Chore(BaseModel):
    chore_id: str
    chore_title: str
    chore_spontaneous_work_enable: bool
    chore_created_ts: float
    chore_archived_ts: float
    _chore_events: Dict[str, ChoreEvent] = PrivateAttr(default_factory=dict)

global_free_cthreads: Dict[str, CThread] = {}
global_free_cthreads_msgdict: Dict[str, Dict[str, CMessage]] = {}

global_chores: Dict[str, Chore] = {}

global_bound_cthreads: Dict[str, CThread] = {}
global_bound_cthreads_msgdict: Dict[str, Dict[str, CThread]] = {}


global_cthread_id_to_cmessages_subs: Dict[str, asyncio.Task] = {}

def cmessages_subscribe_if_not_already(session, cthread_id: str):
    if cthread_id not in global_cthread_id_to_cmessages_subs:
        task = asyncio.create_task(cmessages_sub(session, cthread_id))
        global_cthread_id_to_cmessages_subs[cthread_id] = task


def receive_sub_event(session, line):
    decoded_line = line.decode('utf-8').strip()
    if decoded_line.startswith("data: "):
        decoded_line = decoded_line[6:].strip()
    j = json.loads(decoded_line)
    sub_event = j["sub_event"]
    print(termcolor.colored(sub_event, "red" if sub_event=="cmessage_update" else "magenta"), decoded_line)

    if sub_event == "chore_update":
        chore_obj = Chore(**j["chore_rec"])
        global_chores[chore_obj.chore_id] = chore_obj
    elif sub_event == "chore_delete":
        chore_id = j["chore_id"]
        global_chores.pop(chore_id, None)

    elif sub_event == "chore_event_update":
        chore_event_rec = ChoreEvent(**j["chore_event_rec"])
        belongs_to_chore_id = j["chore_event_belongs_to_chore_id"]
        chore_obj = global_chores[belongs_to_chore_id]
        chore_obj._chore_events[chore_event_rec.chore_event_id] = chore_event_rec
    elif sub_event == "chore_event_delete":
        chore_event_id = j["chore_event_id"]
        belongs_to_chore_id = j["chore_event_belongs_to_chore_id"]
        global_chores[belongs_to_chore_id]._chore_events.pop(chore_event_id, None)

    elif sub_event == "cthread_update":
        cthread_rec = CThread(**j["cthread_rec"])
        if not cthread_rec.cthread_belongs_to_chore_event_id:
            global_free_cthreads[cthread_rec.cthread_id] = cthread_rec
            global_free_cthreads_msgdict.setdefault(cthread_rec.cthread_id, dict())
        else:
            global_bound_cthreads[cthread_rec.cthread_id] = cthread_rec
            global_bound_cthreads_msgdict.setdefault(cthread_rec.cthread_id, cthread_rec)
        cmessages_subscribe_if_not_already(session, cthread_rec.cthread_id)
    elif sub_event == "cthread_delete":
        cthread_id = j["cthread_id"]
        global_free_cthreads.pop(cthread_id, None)
        global_free_cthreads_msgdict.pop(cthread_id, None)
        global_bound_cthreads.pop(cthread_id, None)
        global_bound_cthreads_msgdict.pop(cthread_id, None)

    elif sub_event == "cmessage_update":
        cmessage_rec = CMessage(**j["cmessage_rec"])
        d = global_bound_cthreads_msgdict.get(cmessage_rec.cmessage_belongs_to_cthread_id) or global_free_cthreads_msgdict.get(cmessage_rec.cmessage_belongs_to_cthread_id)
        k = cmessage_key(cmessage_rec.cmessage_belongs_to_cthread_id, cmessage_rec.cmessage_alt, cmessage_rec.cmessage_num)
        if d is not None:
            d[k] = cmessage_rec
        else:
            assert 0
    elif sub_event == "cmessage_delete":
        cmessage_belongs_to_cthread_id = j["cmessage_belongs_to_cthread_id"]
        cmessage_alt = j["cmessage_alt"]
        cmessage_num = j["cmessage_num"]
        d = global_bound_cthreads_msgdict.get(cmessage_rec.cmessage_belongs_to_cthread_id) or global_free_cthreads_msgdict.get(cmessage_rec.cmessage_belongs_to_cthread_id)
        k = cmessage_key(cmessage_belongs_to_cthread_id, cmessage_alt, cmessage_num)
        if d is not None:
            d.pop(k, None)
        else:
            assert 0

    else:
        assert 0, decoded_line


def print_messages(indent, msgdict: Dict[str, CMessage]):
    for message_key, message in msgdict.items():
        mdict = json.loads(message.cmessage_json)
        chat_message = chat_client.Message(**mdict)
        output = termcolor.colored("%s%s role=\"%s\" content=\"%s\"" % (
            indent,
            message_key,
            chat_message.role,
            chat_message.content[:20].replace("\n", "\\n")
        ), "yellow")
        if chat_message.tool_calls:
            output += termcolor.colored(f" tool_calls=\"{chat_message.tool_calls}\"", "red")
        print(output)

def cthread_emojis(cthread: CThread):
    archived_emoji = "🗑️" if cthread.cthread_archived_ts else ""
    error_emoji = ("❌%s" % cthread.cthread_error) if cthread.cthread_error else ""
    new_emoji = "🟡" if cthread.cthread_anything_new else ""
    return f"{archived_emoji}{error_emoji}{new_emoji}"

def print_picture():
    print("\033[H\033[J", end="")
    print("----------------picture--------------")
    print(termcolor.colored("Free CThreads %d" % len(global_free_cthreads), "blue", attrs=["bold"]))
    for cthread_id, cthread in global_free_cthreads.items():
        emojis = cthread_emojis(cthread)
        print(termcolor.colored(f"CThread {cthread_id} {emojis}", "cyan"))
        print_messages("  ", global_free_cthreads_msgdict[cthread_id])

    print(termcolor.colored("Chores / ChoreEvents / CThreads / CMessages", "blue", attrs=["bold"]))
    for chore_id, chore in global_chores.items():
        print(termcolor.colored("Chore: %s has %d events" % (chore_id, len(chore._chore_events)), "magenta"))
        for chore_event_id, chore_event in chore._chore_events.items():
            print(termcolor.colored("  Event %s" % chore_event_id, "green"))
            if chore_event.chore_event_cthread_id:
                cthread = global_bound_cthreads.get(chore_event.chore_event_cthread_id)
                if cthread:
                    emojis = cthread_emojis(cthread)
                    print(termcolor.colored(f"    CThread {cthread.cthread_id} {emojis}", "cyan"))
                    print_messages("      ", global_bound_cthreads_msgdict.get(chore_event.chore_event_cthread_id))
                else:
                    print(termcolor.colored("    CThread %s not found" % chore_event.chore_event_cthread_id, "red"))

    print("----------------/picture-------------")
    print("subscribed cthread:", list(global_cthread_id_to_cmessages_subs.keys()))


async def cmessages_sub(session, cmessage_belongs_to_cthread_id):
    global quit_flag
    try:
        async with session.post(
            f"{BASE_URL}/db_v1/cmessages-sub",
            json={"cmessage_belongs_to_cthread_id": cmessage_belongs_to_cthread_id},
        ) as response:
            if response.status != 200:
                print(termcolor.colored(f"Failed to connect to SSE. Status code: {response.status}", "red"))
                return
            async for line in response.content:
                if not line.strip():
                    continue
                receive_sub_event(session, line)
    except Exception as e:
        print(termcolor.colored(f"Exception occurred: {str(e)}", "red"))
    finally:
        quit_flag = True

async def cthreads_sub(session, quicksearch):
    global quit_flag
    try:
        async with session.post(
            f"{BASE_URL}/db_v1/cthreads-sub",
            json={"limit": 100, "quicksearch": quicksearch},
        ) as response:
            if response.status != 200:
                raise Exception(f"Failed to connect to SSE. Status code: {response.status}")
            async for line in response.content:
                if line.strip():
                    receive_sub_event(session, line)
    except Exception as e:
        print(termcolor.colored(f"Exception occurred: {str(e)}", "red"))
    finally:
        quit_flag = True

async def chores_sub(session, quicksearch: str, limit: int, only_archived: bool):
    global quit_flag
    try:
        async with session.post(
            f"{BASE_URL}/db_v1/chores-sub",
            json={"quicksearch": quicksearch, "limit": limit, "only_archived": only_archived},
        ) as response:
            if response.status != 200:
                raise Exception(f"Failed to connect to SSE. Status code: {response.status}")
            async for line in response.content:
                if line.strip():
                    receive_sub_event(session, line)
    except Exception as e:
        print(termcolor.colored(f"Exception occurred: {str(e)}", "red"))
    finally:
        quit_flag = True

async def main(only_sub=False, quicksearch=""):
    session = aiohttp.ClientSession()

    async def periodic_print():
        while not quit_flag:
            await asyncio.sleep(SLEEP)
            print_picture()

    print_picture()

    tasks = []
    tasks.append(asyncio.create_task(chores_sub(session, quicksearch, limit=100, only_archived=False)))
    tasks.append(asyncio.create_task(cthreads_sub(session, quicksearch)))
    tasks.append(asyncio.create_task(periodic_print()))

    try:
        await asyncio.gather(*tasks)
    finally:
        await session.close()

    print(termcolor.colored("\nTEST OVER", "green", attrs=["bold"]))

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ChoreDB test script")
    parser.add_argument("--only-sub", action="store_true", help="Run only the subscription part")
    parser.add_argument("--quicksearch", type=str, default="", help="Quicksearch term for filtering")
    args = parser.parse_args()

    asyncio.run(main(only_sub=args.only_sub, quicksearch=args.quicksearch))
