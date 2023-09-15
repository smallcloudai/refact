import traceback
import subprocess
import multiprocessing

from typing import List

from refact_vecdb.embeds_api.embed_spads import embed_providers


def execute_child(cmd: List[str]):
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    for line in iter(proc.stdout.readline, b''):
        print(line.decode())


def main():
    cmds = []
    for model in embed_providers:
        cmds.append(['python', '-m', 'self_hosting_machinery.inference.inference_embed', '--model', model])
        cmds.append([*cmds[-1], '--index'])

    processes = []
    for cmd in cmds:
        p = multiprocessing.Process(target=execute_child, args=(cmd,))
        processes.append(p)
        p.start()

    try:
        for p in processes:
            p.join()
    except (Exception, KeyboardInterrupt):
        traceback.print_exc()
        for p in processes:
            p.terminate()


if __name__ == '__main__':
    main()
