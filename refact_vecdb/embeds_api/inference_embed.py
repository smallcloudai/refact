import sys
import logging
import time
import signal
import socket

from refact_scratchpads_no_gpu.stream_results import infserver_session
from refact_scratchpads_no_gpu.stream_results import validate_description_dict
from refact_scratchpads_no_gpu.stream_results import UploadProxy
from refact_scratchpads_no_gpu.stream_results import completions_wait_batch

from refact_vecdb.embeds_api.encoder import VecDBEncoder
from refact_vecdb.embeds_api.embed_spads import embed_providers


quit_flag = False
log = logging.getLogger("MODEL").info


def worker_loop(
        model_name: str,
        index: bool,
        compile_only: bool = False,
):
    if model_name not in embed_providers:
        log(f"STATUS model \"{model_name}\" not found")
        if compile_only:
            return
        log("will sleep for 5 minutes and then exit, to slow down service restarts")
        wake_up_ts = time.time() + 300
        while time.time() < wake_up_ts and not quit_flag:
            time.sleep(1)
        raise RuntimeError(f"unknown model \"{model_name}\"")
    log("STATUS loading model")

    enc_model = VecDBEncoder(model_name)

    class DummyUploadProxy:
        def upload_result(*args, **kwargs):
            pass

        def check_cancelled(*args, **kwargs):
            return set()

    dummy_calls = [
        {
            'id': 'emb-legit-42',
            'function': 'completion',
            'files': [{'name': 'abc.py', 'text': 'print("hello, world")'}],
            'created': time.time(),
        }
    ]
    log("STATUS test batch")
    enc_model.infer(dummy_calls[0], DummyUploadProxy, {}, log)
    if compile_only:
        return

    model = model_name if not index else f'{model_name}_index'
    log("STATUS serving %s" % model)
    req_session = infserver_session()
    description_dict = validate_description_dict(
        f'{model}_{socket.getfqdn()}',
        "account_name",
        model=model, B=1, max_thinking_time=10,
    )
    upload_proxy = UploadProxy(upload_q=None, cancelled_q=None)
    upload_proxy.start_upload_result_daemon()

    while not quit_flag:
        upload_proxy.keepalive()
        upload_proxy.cancelled_reset()
        retcode, request_batch = completions_wait_batch(
            req_session, description_dict, verbose=False)
        ts_arrived = time.time()
        if retcode == "OK":
            for request in request_batch:
                upload_proxy_args = {
                    "description_dict": description_dict,
                    "original_batch": [request],
                    "idx_updated": [0],
                    "tokens": None,
                    "ts_arrived": ts_arrived,
                    "ts_batch_started": time.time(),
                    "ts_prompt": 0,
                    "ts_first_token": 0,
                    "ts_batch_finished": 0,
                }
                enc_model.infer(request, upload_proxy, upload_proxy_args, log)
        elif retcode == "WAIT":
            pass
        else:
            time.sleep(10)

    upload_proxy.stop()
    log("clean shutdown")


def catch_sigkill(signum, frame):
    sys.stderr.write("caught SIGUSR1\n")
    sys.stderr.flush()
    global quit_flag
    quit_flag = True


if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--model", type=str)
    parser.add_argument("--index", action="store_true")
    parser.add_argument("--compile", action="store_true")
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s MODEL %(message)s',
        datefmt='%H:%M:%S',
        handlers=[logging.StreamHandler(stream=sys.stderr)]
    )

    signal.signal(signal.SIGUSR1, catch_sigkill)

    worker_loop(args.model, args.index, args.compile)
