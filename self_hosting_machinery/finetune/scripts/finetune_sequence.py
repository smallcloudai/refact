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
@click.option('--project', default='', help='Project name')
@click.argument('the_rest_of_args', nargs=-1)
def main(filter_only, project, the_rest_of_args):
    # print("filter_only: %s, project: %s, the_rest_of_args: %s" % (filter_only, project, the_rest_of_args))
    if not filter_only:
        os.environ["LORA_LOGDIR"] = time.strftime("lora-%Y%m%d-%H%M%S")
    else:
        os.environ["LORA_LOGDIR"] = "NO_LOGS"
    subprocess.check_call([sys.executable, "-m", "self_hosting_machinery.finetune.scripts.process_uploaded_files", "--project", project])
    subprocess.check_call([sys.executable, "-m", "self_hosting_machinery.finetune.scripts.finetune_filter", "--project", project])
    if not filter_only:
        subprocess.check_call([sys.executable, "-m", "self_hosting_machinery.finetune.scripts.finetune_train", "--project", project, *the_rest_of_args])

if __name__ == '__main__':
    signal.signal(signal.SIGUSR1, catch_sigusr1)
    try:
        main.main(sys.argv[1:], standalone_mode=False)
    except subprocess.CalledProcessError as e:
        print("finetune_sequence: %s" % e)
        sys.exit(1)
