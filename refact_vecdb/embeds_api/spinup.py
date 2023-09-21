import time
import multiprocessing

from typing import List, Dict

from refact_vecdb.embeds_api.wrapper import VDBEmbeddingsAPI

from refact_vecdb.common.context import CONTEXT as C
from refact_vecdb.common.crud import get_all_providers

from refact_vecdb.embeds_api.inference_embed import worker_loop


__all__ = ['spinup_models']
multiprocessing.set_start_method("spawn", force=True)


def kill_process(process):
    process.terminate()
    process.kill()
    process.join()
    process.close()


def spinup_models():
    C.processes.setdefault('models', {})
    print(f'models: {list(C.processes["models"].keys())}')
    if providers := get_all_providers():
        processes = _spinup_models(
            models=providers,
            processes=C.processes['models'],
        )
        C.processes['models'].update(processes)


def _spinup_models(
        models,
        processes: Dict[str, multiprocessing.Process],
):
    # def remove_unused_processes():
    #     models_present = set(processes.keys())
    #     models_given = set()
    #     for model in models:
    #         for suffix in ['', '_index']:
    #             models_given.add(f'{model}{suffix}')
    #     models_remove = models_present - models_given
    #     for model in models_remove:
    #         if processes.get(model):
    #             pid = processes[model].pid
    #             kill_process(processes[model])
    #             del processes[model]
    #             print(f'PID: {pid}: Model {model} terminated')

    def spinup():
        ctx = multiprocessing.get_context('spawn')
        for model in models:
            for suffix in ['', '_index']:
                model_name = model + suffix
                is_index = True if suffix else False
                if processes.get(model_name) and processes[model_name].is_alive():
                    print(f'Model {model}{suffix} already running')
                    return
                if processes.get(model_name) and not processes[model_name].is_alive():
                    kill_process(processes[model_name])

                p = ctx.Process(target=worker_loop, args=(model, is_index))
                processes[model_name] = p
                p.start()
                print(f'PID: {p.pid}: Model {model}{suffix} started')

    def test_models_are_running():
        api = VDBEmbeddingsAPI()
        for model in models:
            for suffix in ['', '_index']:
                while True:
                    try:
                        is_index = True if suffix else False
                        res = list(api.create({'name': 'test', 'text': 'test'}, provider=model, is_index=is_index))
                        res = res[0]
                        assert isinstance(res, dict)
                    except Exception: # noqa
                        print(f'Model {model}{suffix} is not ready yet...')
                        time.sleep(5)
                    else:
                        print(f'Model {model}{suffix} is ready')
                        break

    # TODO: does not work properly; causes memory leak
    # if remove_unused:
    #     remove_unused_processes()
    spinup()
    test_models_are_running()
    return processes


if __name__ == '__main__':
    procs = {}
    # _spinup_models(embed_providers, procs)
    # while True:
    #     time.sleep(10)
