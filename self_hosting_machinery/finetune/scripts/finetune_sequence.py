import click
import os
import signal
import subprocess
import sys
import time
import psutil

def catch_sigusr1(signum, frame):
    print("catched SIGUSR1")
    current_process = psutil.Process()
    for child in current_process.children(recursive=False):
        os.kill(child.pid, signal.SIGUSR1)

@click.command(context_settings=dict(ignore_unknown_options=True, allow_extra_args=True))
@click.option('--filter-only', is_flag=True, help='Filter only flag')
@click.option('--pname', default='', help='Project name')
@click.option('--run_id', default='', help='Finetune run name')
@click.argument('the_rest_of_args', nargs=-1)
def main(filter_only, pname, run_id, the_rest_of_args):
    # print("filter_only: %s, pname: %s, the_rest_of_args: %s" % (filter_only, pname, the_rest_of_args))
    if not filter_only:
        if not run_id:
            run_id = time.strftime("lora-%Y%m%d-%H%M%S")
        os.environ["LORA_LOGDIR"] = run_id
    else:
        os.environ["LORA_LOGDIR"] = "NO_LOGS"
    # subprocess.check_call([sys.executable, "-m", "self_hosting_machinery.finetune.scripts.process_uploaded_files", "--pname", pname])
    cuda_visible_devices = os.getenv("CUDA_VISIBLE_DEVICES", "")
    ngpus = len(cuda_visible_devices.split(","))
    if ngpus > 1:
        # python -m torch.distributed.launch --nproc_per_node=8 ~/code/refact/self_hosting_machinery/finetune/scripts/finetune_train.py
        my_dir = os.path.dirname(os.path.realpath(__file__))
        finetune_train = os.path.join(my_dir, "finetune_train.py")
        subprocess.check_call([sys.executable, "-m", "torch.distributed.launch", "--nproc_per_node=%d" % ngpus, finetune_train, "--pname", pname, "--run_id", run_id, *the_rest_of_args])
    else:
        subprocess.check_call([sys.executable, "-m", "self_hosting_machinery.finetune.scripts.finetune_train", "--pname", pname, "--run_id", run_id, *the_rest_of_args])
    # TODO: gpus > 1

if __name__ == '__main__':
    signal.signal(signal.SIGUSR1, catch_sigusr1)
    try:
        main.main(sys.argv[1:], standalone_mode=False)
    except subprocess.CalledProcessError as e:
        print("finetune_sequence: %s" % e)
        sys.exit(1)
