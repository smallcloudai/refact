import click
import os
import signal
import subprocess
import sys
import psutil

def catch_sigusr1(signum, frame):
    print("catched SIGUSR1")
    current_process = psutil.Process()
    for child in current_process.children(recursive=False):
        os.kill(child.pid, signal.SIGUSR1)

@click.command(context_settings=dict(ignore_unknown_options=True, allow_extra_args=True))
@click.option('--pname', default='', help='Project name')
@click.argument('the_rest_of_args', nargs=-1)
def main(pname, the_rest_of_args):
    cuda_visible_devices = os.getenv("CUDA_VISIBLE_DEVICES", "")
    ngpus = len(cuda_visible_devices.split(","))
    if ngpus > 1:
        # python -m torch.distributed.launch --nproc_per_node=8 ~/code/refact/self_hosting_machinery/finetune/scripts/finetune_train.py
        finetune_train = os.path.join(os.path.dirname(os.path.realpath(__file__)), "finetune_train.py")
        subprocess.check_call([sys.executable, "-m", "torch.distributed.launch", "--nproc_per_node=%d" % ngpus, finetune_train, "--pname", pname, *the_rest_of_args])
    else:
        subprocess.check_call([sys.executable, "-m", "self_hosting_machinery.finetune.scripts.finetune_train", "--pname", pname, *the_rest_of_args])

if __name__ == '__main__':
    signal.signal(signal.SIGUSR1, catch_sigusr1)
    try:
        main.main(sys.argv[1:], standalone_mode=False)
    except subprocess.CalledProcessError as e:
        print("finetune_sequence: %s" % e)
        sys.exit(1)
