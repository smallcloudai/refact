import os
import socket
import sys
import time
import json
import datetime
import traceback
import signal
import logging

import importlib
import asyncio

from smallcloud import inference_server_async


DEBUG = int(os.environ.get("DEBUG", "0"))
ROUTINES = int(os.environ.get("ROUTINES", "0"))


gpt4_functions = {
    "make-code-shorter-gpt4":    "refact_scratchpads_no_gpu:ScratchpadMakeCodeShorterGPT4",
    "fix-bug-gpt4":              "refact_scratchpads_no_gpu:ScratchpadFixBugGPT4",
    "explain-code-block-gpt4":   "refact_scratchpads_no_gpu:ScratchpadExplainCodeBlockGPT4",

    "completion-gpt4":           "refact_scratchpads_no_gpu:ScratchpadCompletionGPT4",
    "free-chat-gpt4":            "refact_scratchpads_no_gpu:GptChat",

    # UNFINISHED:
    # "detect-bugs":               "refact_scratchpads_no_gpu:ScratchpadDetectBugsHighlight",
    # "detect-vulnerabilities":    "refact_scratchpads_no_gpu:ScratchpadDetectVulnerabilitiesHighlight",
    # "code-review":               "refact_scratchpads_no_gpu:ScratchpadCodeReviewHighlight",
    # "detect-bugs-highlight-gpt4": "refact_scratchpads_no_gpu:ScratchpadFixBugsHighlightGPT4",
}

gpt35_functions = {
    "make-code-shorter":         "refact_scratchpads_no_gpu:ScratchpadMakeCodeShorter",
    "make-code-shorter-gpt3.5":  "refact_scratchpads_no_gpu:ScratchpadMakeCodeShorter",
    "fix-bug":                   "refact_scratchpads_no_gpu:ScratchpadFixBug",
    "fix-bug-gpt3.5":            "refact_scratchpads_no_gpu:ScratchpadFixBug",
    "explain-code-block":        "refact_scratchpads_no_gpu:ScratchpadExplainCodeBlock",
    "explain-code-block-gpt3.5": "refact_scratchpads_no_gpu:ScratchpadExplainCodeBlock",

    # 3.5 only
    "add-console-logs":          "refact_scratchpads_no_gpu:ScratchpadAddConsoleLogs",
    "add-console-logs-gpt3.5":   "refact_scratchpads_no_gpu:ScratchpadAddConsoleLogs",
    "precise-naming":            "refact_scratchpads_no_gpu:ScratchpadPreciseNaming",
    "precise-naming-gpt3.5":     "refact_scratchpads_no_gpu:ScratchpadPreciseNaming",
    "comment-each-line":         "refact_scratchpads_no_gpu:ScratchpadCommentEachLine",
    "comment-each-line-gpt3.5":  "refact_scratchpads_no_gpu:ScratchpadCommentEachLine",

    "completion-gpt3.5":         "refact_scratchpads_no_gpu:ScratchpadCompletion",
    "free-chat":                 "refact_scratchpads_no_gpu:GptChat",
    "free-chat-gpt3.5":          "refact_scratchpads_no_gpu:GptChatFunctions",  # replace with GptChat
    # "db-chat-gpt3.5func":          "gpt_toolbox.gpt_chat_functions_spad:GptChat",

    # UNFINISHED
    # "detect-bugs-highlight-gpt3.5": "gpt_toolbox.gpt_toolbox:ScratchpadFixBugsHighlight",
}


supported_models = {
    "longthink/stable": {
        "functions": {
            **gpt4_functions,
            **gpt35_functions
        }
    },
}


for mod in ["debug", "experimental"]:
    supported_models["longthink/" + mod] = supported_models["longthink/stable"]


host = socket.getfqdn()
quit_flag = False


def dump_problematic_call(stacktrace: str, stacktrace_short: str, suspicious_call):
    if suspicious_call and not DEBUG:
        # not DEBUG means in production, save it to disk to check out later
        ymd = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        dump_path = f'./{ymd}_infserver_no_gpu_stacktrace.dump'
        with open(dump_path, 'w') as f:
            f.write(f"{host} caught exception:\n{stacktrace}")
            f.flush()
            f.write(json.dumps(suspicious_call))
        sys.stdout.write("'%s' DUMP SAVED TO %s\n" % (stacktrace_short, dump_path))
        sys.stdout.flush()
    elif suspicious_call:
        # if DEBUG, just print the call that caused the problem
        sys.stdout.write(json.dumps(suspicious_call))
        sys.stdout.flush()


def except_hook(exctype, value, tb, suspicious_call=None):
    msg = "".join(traceback.format_exception(exctype, value, tb, limit=10))
    sys.stderr.write(msg)
    sys.stderr.flush()
    if exctype == KeyboardInterrupt:
        quit()
    dump_problematic_call(
        "".join(traceback.format_exception(exctype, value, tb, limit=None, chain=True)),
        f"{exctype.__name__}: {value}",
        suspicious_call
    )


async def handle_single_batch(routine_n, my_desc, model_dict, calls_unfiltered):
    ts_arrived = time.time()
    uproxy = inference_server_async.UploadAsync()
    upload_task = asyncio.create_task(uproxy.upload_results_coroutine())
    calls = []
    def logger(*args):
        msg = " ".join(map(str, args))
        msg = "R%04d" % routine_n + " " + msg
        inference_server_async.logger.info(msg)
    try:
        scratchpads = []
        for ci, call in enumerate(calls_unfiltered):
            function = call.get("function", "completion")
            import_str = model_dict["functions"].get(function, None)
            if import_str is None:
                logger("function '%s' is not supported in model '%s'" % (function, call["model"]))
                continue
            import_mod, import_class = import_str.rsplit(":", 1)
            mod = importlib.import_module(import_mod)
            Class = getattr(mod, import_class, None)
            if Class is None:
                logger("module '%s', class '%s' not found" % (import_mod, import_class))
                continue
            logger("running '%s' using %s" % (function, import_class))
            calls.append(call)
            spad = Class(logger=logger, **call)
            scratchpads.append(spad)

        ts_batch_started = time.time()
        # for i in range(len(calls)):
        #     _prompt = scratchpads[i].prompt()
        ts_prompt = time.time()
        ts_first_token = time.time()

        for call_n, (call, spad) in enumerate(zip(calls, scratchpads)):
            async for files_dict in spad.completion():
                assert isinstance(files_dict, dict), f'expected dict, got {type(files_dict)}'
                cancelled_idset = uproxy.check_cancelled()
                if call["id"] in cancelled_idset:
                    spad.finish_reason = "cancelled"
                uproxy.upload_result(
                    my_desc,
                    [call],
                    ts_arrived=ts_arrived,
                    ts_batch_started=ts_batch_started,
                    ts_prompt=ts_prompt,
                    ts_first_token=ts_first_token,
                    ts_batch_finished=time.time() if spad.finish_reason else 0,
                    idx_updated=[call_n],
                    files=[files_dict],
                    tokens=None,
                    finish_reason=[spad.finish_reason],
                    status=("completed" if spad.finish_reason else "in_progress"),
                    more_toplevel_fields=[spad.toplevel_fields()],
                )
                if call["id"] in cancelled_idset:
                    break
    except Exception as e:
        except_hook(type(e), e, e.__traceback__, calls[0] if len(calls) else None)
    finally:
        await uproxy.shutdown_coroutine()
        await upload_task
        await uproxy.close_session()
        uproxy.cancelled_reset()
        upload_task = None



def catch_sigkill(signum, frame):
    print("catched SIGKILL")
    global quit_flag
    quit_flag = True


async def do_the_serving(
    longthink_variant: str,
    routine_n: int,
):
    aio_session = inference_server_async.infserver_async_session()
    infmod_guid = longthink_variant + "_" + host + "_%04i" % routine_n
    infmod_guid = infmod_guid.replace("-", "_")
    inference_server_async.logger.info(f'infmod_guid: {infmod_guid}')
    while not quit_flag:
        model_dict = supported_models[longthink_variant]
        my_desc = inference_server_async.validate_description_dict(
            infeng_instance_guid=infmod_guid,
            account="engineer",
            model=longthink_variant,
            B=1,
            max_thinking_time=10,
        )
        retcode, calls_unfiltered = await inference_server_async.completions_wait_batch(aio_session, my_desc)
        if retcode == "WAIT":
            continue
        if retcode != "OK":
            inference_server_async.logger.warning("server retcode %s" % retcode)
            await asyncio.sleep(5)
            continue
        await handle_single_batch(routine_n, my_desc, model_dict, calls_unfiltered)
    await aio_session.close()
    inference_server_async.logger.info("clean shutdown")


def main():
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s NOGPU %(message)s',
        datefmt='%Y%m%d %H:%M:%S',
        handlers=[logging.StreamHandler(stream=sys.stderr)])

    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("longthink_variant", type=str, default='longthink/stable')
    parser.add_argument("-k", "--openai_key", type=str)
    parser.add_argument("-w", "--workers", type=int, default=1)
    parser.add_argument("-p", '--port', type=int, default=8008)
    parser.add_argument("--selfhosted", action="store_true")
    args = parser.parse_args()

    if args.selfhosted:
        from smallcloud import inference_server
        inference_server.override_urls(f"http://127.0.0.1:{args.port}/infengine-v1/")

    if not (args.openai_key or os.environ.get('OPENAI_API_KEY')):
        raise RuntimeError("set OPENAI_API_KEY or use --openai_key")

    if args.openai_key:
        import openai
        openai.api_key = args.openai_key

    sys.excepthook = except_hook
    signal.signal(signal.SIGUSR1, catch_sigkill)

    workers: int = max(1, args.workers) if not DEBUG else 1
    asyncio.get_event_loop().run_until_complete(asyncio.gather(*[
        do_the_serving(args.longthink_variant, routine_n)
        for routine_n in range(workers)
    ]))


if __name__ == "__main__":
    main()
