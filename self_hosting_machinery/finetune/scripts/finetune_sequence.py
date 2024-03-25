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
@click.option('--pname', required=True, help='Project name')
@click.argument('args', nargs=-1)
def main(pname, args):
    cuda_visible_devices = os.getenv("CUDA_VISIBLE_DEVICES", "")
    num_gpus = len(cuda_visible_devices.split(","))
    cmd = [sys.executable]
    if num_gpus > 1:
        # this tries to run the same as:
        # CUDA_VISIBLE_DEVICES=4,5,6,7 torchrun --nproc_per_node=4 ~/code/refact/self_hosting_machinery/finetune/scripts/finetune_train.py --run_id helloworld-20240320-000003 --pname Refact --model_name Refact/1.6B --low_gpu_mem_mode False
        port = 20000 + hash(cuda_visible_devices) % 1000
        cmd = ["torchrun", "--master-port", str(port), f"--nproc_per_node={num_gpus}"]
    finetune_train_script = os.path.join(os.path.dirname(os.path.realpath(__file__)), "finetune_train.py")
    subprocess.check_call([*cmd, finetune_train_script, "--pname", pname, *args])


if __name__ == '__main__':
    signal.signal(signal.SIGUSR1, catch_sigusr1)
    try:
        main.main(sys.argv[1:], standalone_mode=False)
    except subprocess.CalledProcessError as e:
        print("finetune_sequence: %s" % e)
        sys.exit(1)
