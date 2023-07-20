import sys
import logging
import time
import signal
import socket

from known_models_db.refact_known_models import models_mini_db
from refact_scratchpads_no_gpu.stream_results import infserver_session
from refact_scratchpads_no_gpu.stream_results import validate_description_dict
from refact_scratchpads_no_gpu.stream_results import UploadProxy
from refact_scratchpads_no_gpu.stream_results import completions_wait_batch

from self_hosting_machinery.inference import InferenceLegacy
from self_hosting_machinery.inference import InferenceHF


quit_flag = False
log = logging.getLogger("MODEL").info


def worker_loop(model_name: str, cpu: bool, load_lora: str, compile: bool):
    if model_name not in models_mini_db:
        log(f"STATUS model \"{model_name}\" not found")
        if compile:
            return
        log("will sleep for 5 minutes and then exit, to slow down service restarts")
        wake_up_ts = time.time() + 300
        while time.time() < wake_up_ts and not quit_flag:
            time.sleep(1)
        raise RuntimeError(f"unknown model \"{model_name}\"")
    log("STATUS loading model")

    model_dict = models_mini_db[model_name]
    if "model_class" in model_dict:
        inference_model = InferenceLegacy(
            model_name=model_name,
            model_dict=model_dict,
            force_cpu=cpu,
            load_lora=load_lora)
    else:
        inference_model = InferenceHF(
            model_name=model_name,
            model_dict=model_dict)

    class DummyUploadProxy:
        def upload_result(*args, **kwargs):
            pass
        def check_cancelled(*args, **kwargs):
            return set()
    dummy_calls = [
        {
            'temperature': 0.8, 'top_p': 0.95, 'max_tokens': 40, 'id': 'comp-wkCX57Le8giP-1337', 'object': 'text_completion_req',
            'function': 'completion',
            'echo': False,
            'stop_tokens': [],
            'prompt': 'Hello world',
            'created': time.time(),
        }
    ]
    log("STATUS test batch")
    inference_model.infer(dummy_calls[0], DummyUploadProxy, {})
    if compile:
        return

    log("STATUS serving %s" % model_name)
    req_session = infserver_session()
    description_dict = validate_description_dict(
        model_name + "_" + socket.getfqdn(),
        "account_name",
        model=model_name, B=1, max_thinking_time=10,
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
            inference_model.lora_switch_according_to_config()
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
                inference_model.infer(request, upload_proxy, upload_proxy_args)
        elif retcode == "WAIT":
            # Normal, no requests
            pass
        else:
            # No connectivity, connection refused, other errors go there
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
    parser.add_argument("--cpu", action="store_true")
    parser.add_argument("--load-lora")
    parser.add_argument("--compile", action="store_true", help="download and compile triton kernels, quit")
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s MODEL %(message)s',
        datefmt='%Y%m%d %H:%M:%S',
        handlers=[logging.StreamHandler(stream=sys.stderr)])

    signal.signal(signal.SIGUSR1, catch_sigkill)

    worker_loop(args.model, args.cpu, args.load_lora, compile=args.compile)
