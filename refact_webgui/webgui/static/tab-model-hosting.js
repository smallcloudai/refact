import {general_error} from './error.js';
import {get_finetune_config_and_runs} from './tab-finetune.js';
import {add_finetune_selectors_factory, update_checkpoints_list, finetune_info_factory} from './utils/tab-model-hosting-utils.js';


let gpus_popup = false;
let models_data = null;
let finetune_configs_and_runs;
let force_render_models_assigned = false;

function update_finetune_configs_and_runs() {
    get_finetune_config_and_runs().then((data) => {
        if (!data) {
            return;
        }
        finetune_configs_and_runs = data;
    })
}

function get_gpus() {
    fetch("/tab-host-have-gpus")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        render_gpus(data);
    })
   .catch(function(error) {
        console.log('tab-host-have-gpus',error);
        general_error(error);
    });
}

function finetune_switch_activate(finetune_model, mode, run_id, checkpoint) {
    let send_this = {
        "model": finetune_model,
        "mode": mode,
        "run_id": run_id,
        "checkpoint": checkpoint,
    }
    return fetch("/tab-host-modify-loras", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(send_this)
    })
    .then(function (response) {
        if (!response.ok) {
            return response.json().then(function(json) {
                throw new Error(json.detail);
            });
        }
        return response.ok;
    })
    .catch(function (error) {
        console.log('tab-finetune-activate',error);
        general_error(error);
        return false;
    });
}

function render_gpus(gpus) {
    if(gpus_popup) { return; }
    if(gpus.gpus.length == 0) {
        document.querySelector('.gpus-pane').style.display = 'none';
    } else {
        document.querySelector('.gpus-pane').style.display = 'div';
    }
    const gpus_list = document.querySelector('.gpus-list');
    gpus_list.innerHTML = '';
    gpus.gpus.forEach(element => {
        const row = document.createElement('div');
        row.classList.add('gpus-item');
        row.setAttribute('gpu',element.id);
        const gpu_wrapper = document.createElement('div');
        gpu_wrapper.classList.add('gpus-content');
        const gpu_name = document.createElement("h3");
        gpu_name.classList.add('gpus-title');
        const gpu_image = document.createElement("div");
        gpu_image.classList.add('gpus-card');
        const gpu_mem = document.createElement("div");
        gpu_mem.classList.add('gpus-mem');
        const gpu_temp = document.createElement("div");
        gpu_temp.classList.add('gpus-temp');
        const used_gb = format_memory(element.mem_used_mb);
        const total_gb = format_memory(element.mem_total_mb);
        const used_mem = Math.round(element.mem_used_mb / (element.mem_total_mb / 100));
        gpu_name.innerHTML = element.name;
        gpu_mem.innerHTML = `<b>Mem</b><div class="gpus-mem-wrap"><div class="gpus-mem-bar"><span style="width: ${used_mem}%"></span></div>${used_gb}/${total_gb} GB</div>`;
        if (element.temp_celsius < 0) {
            gpu_temp.innerHTML = `<b>Temp</b> N/A`;
        } else {
            gpu_temp.innerHTML = `<b>Temp</b>` + element.temp_celsius + '°C';
        }
        row.appendChild(gpu_image);
        gpu_wrapper.appendChild(gpu_name);
        gpu_wrapper.appendChild(gpu_mem);
        gpu_wrapper.appendChild(gpu_temp);
        element.statuses.forEach(status => {
            const gpu_command = document.createElement("div");
            gpu_command.classList.add('gpus-command');
            const gpu_status = document.createElement("div");
            gpu_status.classList.add('gpus-status');
            gpu_command.innerHTML = `<span class="gpus-current-status">${status.status}</span>`;
            gpu_status.innerHTML += `<div><b>Command</b>${status.command}</div>`;
            gpu_status.innerHTML += `<div><b>Status</b>${status.status}</div>`;
            gpu_command.appendChild(gpu_status);
            gpu_command.addEventListener('mouseover',function(e) {
                gpus_popup = true;
                this.querySelector('.gpus-status').classList.add('gpus-status-visible');
            });
            gpu_command.addEventListener('mouseout',function(e) {
                gpus_popup = false;
                this.querySelector('.gpus-status').classList.remove('gpus-status-visible');
            });
            if(!status.status || status.status === '') {
                gpu_command.classList.add('gpus-status-invisible');
            }
            gpu_wrapper.appendChild(gpu_command);
        });

        row.appendChild(gpu_wrapper);
        gpus_list.appendChild(row);
    });
}

function integration_switch_init(integration_checkbox_id, checked) {
    const enable_switch = document.getElementById(integration_checkbox_id);
    enable_switch.removeEventListener('change', save_model_assigned);
    enable_switch.checked = checked;
    enable_switch.addEventListener('change', save_model_assigned);
}

function get_models()
{
    fetch("/tab-host-models-get")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        models_data = data;
        render_models_assigned(data.model_assign);

        integration_switch_init('enable_chat_gpt', models_data['openai_api_enable']);
        integration_switch_init('enable_anthropic', models_data['anthropic_api_enable']);

        const more_gpus_notification = document.querySelector('.model-hosting-error');
        if(data.hasOwnProperty('more_models_than_gpus') && data.more_models_than_gpus) {
            more_gpus_notification.classList.remove('d-none');
        } else {
            more_gpus_notification.classList.add('d-none');
        }
        const required_memory_exceed_available = document.querySelector('.model-memory-error');
        if(data.hasOwnProperty('required_memory_exceed_available') && data.required_memory_exceed_available) {
            required_memory_exceed_available.classList.remove('d-none');
        } else {
            required_memory_exceed_available.classList.add('d-none');
        }
    })
    .catch(function(error) {
        console.log('tab-host-models-get', error);
        general_error(error);
    });
}

function save_model_assigned() {
    const openai_enable = document.querySelector('#enable_chat_gpt');
    const anthropic_enable = document.querySelector('#enable_anthropic');
    const data = {
        model_assign: {
            ...models_data.model_assign,
        },
        completion: models_data.completion ? models_data.completion : "",
        openai_api_enable: openai_enable.checked,
        anthropic_api_enable: anthropic_enable.checked,
    };
    fetch("/tab-host-models-assign", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(data)
    })
    .then(function (response) {
        get_models();
    })
   .catch(function (error) {
        console.log('tab-host-models-assign',error);
        general_error(error);
    });
}

function set_finetune_info_into_state(model_name, is_enabled) {
    const finetune_info = document.querySelector(`.model-finetune-info[data-model="${model_name}"]`);
    if (is_enabled) {
        finetune_info.style.pointerEvents = 'auto';
        finetune_info.style.opacity = '1';
    } else {
        finetune_info.style.pointerEvents = 'none';
        finetune_info.style.opacity = '0.5';
    }
}

function render_models_assigned(models) {
    const models_info = models_data.models.reduce(function(obj, item) {
      obj[item["name"]] = item;
      return obj;
    }, {});
    const models_table = document.querySelector('.table-assigned-models tbody');
    models_table.innerHTML = '';
    for(let index in models) {
        const row = document.createElement('tr');
        row.setAttribute('data-model',index);
        let model_name = document.createElement("td");
        model_name.style.width = "20%";
        let completion = document.createElement("td");
        completion.style.width = "15%";
        let finetune_info = document.createElement("td");
        finetune_info.style.width = "35%";
        let select_gpus = document.createElement("td");
        select_gpus.style.width = "15%";
        let gpus_share = document.createElement("td");
        gpus_share.style.width = "10%";
        let del = document.createElement("td");
        del.style.width = "5%";

        model_name.textContent = index;
        finetune_info.classList.add('model-finetune-info');
        finetune_info.dataset.model = index;

        if (models_info[index].hasOwnProperty('has_completion') && models_info[index].has_completion) {
            const completion_input = document.createElement("input");
            completion_input.setAttribute('type','radio');
            completion_input.setAttribute('name','completion-radio-button');
            completion_input.setAttribute('value',index);
            if (models_data.completion === index) {
                completion_input.checked = true;
            }
            completion_input.setAttribute('model',index);
            completion_input.addEventListener('change', function() {
                models_data.completion = this.value;
                save_model_assigned();
            });
            completion.appendChild(completion_input);
        }
        let finetune_runs = [];
        if (finetune_configs_and_runs) {
            finetune_runs = finetune_configs_and_runs.finetune_runs;
        } else {
            force_render_models_assigned = true;
        }
        finetune_info_factory(models_data, models_info, finetune_info, finetune_runs, index);

         if (models_info[index].hasOwnProperty('has_sharding') && models_info[index].has_sharding) {
            const select_gpus_div = document.createElement("div");
            select_gpus_div.setAttribute("class", "btn-group btn-group-sm");
            select_gpus_div.setAttribute("role", "group");
            select_gpus_div.setAttribute("aria-label", "basic radio toggle button group");

            [1, 2, 4].forEach((gpus_shard_n) => {
                const input_name = `gpu-${index}`;
                const input_id = `${input_name}-${gpus_shard_n}`;

                const input = document.createElement("input");
                input.setAttribute("type", "radio");
                input.setAttribute("class", "gpu-switch btn-check");
                input.setAttribute("name", input_name);
                input.setAttribute("id", input_id);
                if (models_data.model_assign[index].gpus_shard === gpus_shard_n) {
                    input.checked = true;
                }

                const label = document.createElement("label");
                label.setAttribute("class", "btn btn-outline-primary");
                label.setAttribute("for", input_id);
                label.innerHTML = gpus_shard_n;

                input.addEventListener('change', () => {
                    models_data.model_assign[index].gpus_shard = gpus_shard_n;
                    save_model_assigned();
                });

                select_gpus_div.appendChild(input);
                select_gpus_div.appendChild(label);
            });
            select_gpus.appendChild(select_gpus_div);
        }

        const gpus_checkbox = document.createElement("input");
        gpus_checkbox.setAttribute('type','checkbox');
        gpus_checkbox.setAttribute('value',index);
        gpus_checkbox.setAttribute('name',`share-${index}`);
        gpus_checkbox.classList.add('form-check-input');
        if(models_data.model_assign[index].share_gpu) {
            gpus_checkbox.checked = true;
        } 
        gpus_checkbox.addEventListener('change', function() {
            if(this.checked) {
                models_data.model_assign[index].share_gpu = true;
            } else {
                models_data.model_assign[index].share_gpu = false;
            }
            save_model_assigned();
        });
        gpus_share.appendChild(gpus_checkbox);

        const del_button = document.createElement("button");
        del_button.innerHTML = `<i class="bi bi-trash3-fill"></i>`;
        del_button.dataset.model = index;
        del_button.addEventListener('click', function() {
            delete models_data.model_assign[index];
            save_model_assigned();
        });
        del_button.classList.add('model-remove','btn','btn-outline-danger');
        del.appendChild(del_button);

        row.appendChild(model_name);
        row.appendChild(completion);
        row.appendChild(finetune_info);
        row.appendChild(select_gpus);
        row.appendChild(gpus_share);
        row.appendChild(del);
        models_table.appendChild(row);
    }

    finetune_delete_events();

    document.querySelectorAll(".add-finetune-btn").forEach(element => {
        element.addEventListener("click", (event) => {
            const target = event.currentTarget;
            target.hidden = true;

            let finetune_info = document.querySelector(`.model-finetune-info[data-model="${target.dataset.model}"]`);

            let finetune_selectors = add_finetune_selectors_factory(finetune_configs_and_runs, models_info, target.dataset.model);
            finetune_info.insertBefore(finetune_selectors, target);

            let finetune_add_btn = document.querySelector("#finetune-select-run-btn-add");
            finetune_add_btn.disabled = true;

            let finetune_select_run_btn = document.getElementById('add-finetune-select-run-btn');

            let run_menu = document.getElementById('add-finetune-select-run-menu');

            let finetune_select_checkpoint_btn = document.getElementById('add-finetune-select-checkpoint-btn');
            let checkpoint_menu = document.getElementById('add-finetune-select-checkpoint-menu');

            let toggle_menu_display = (menu) => {
                menu.style.display = (menu.style.display === 'none' || menu.style.display === '') ? 'block' : 'none';
            }

            finetune_select_run_btn.addEventListener('click', (event) => {
                toggle_menu_display(run_menu);
            });

            finetune_select_checkpoint_btn.addEventListener('click', (event) => {
                toggle_menu_display(checkpoint_menu);
            });

            document.addEventListener('click', function(event) {
                if (!run_menu.contains(event.target) && event.target !== finetune_select_run_btn) {
                    run_menu.style.display = 'none';
                }
                if (!checkpoint_menu.contains(event.target) && event.target !== finetune_select_checkpoint_btn) {
                    checkpoint_menu.style.display = 'none';
                }
            });

            document.querySelectorAll('.add-finetune-select-run-di').forEach(element => {
               element.addEventListener('click', function(e) {
                   finetune_select_run_btn.innerText = `${e.currentTarget.innerText}`;
                   finetune_select_run_btn.dataset.run = e.currentTarget.dataset.run;
                   run_menu.style.display = 'none';

                   update_checkpoints_list(finetune_configs_and_runs, finetune_select_checkpoint_btn, e.currentTarget.dataset.run, checkpoint_menu);

                   finetune_select_checkpoint_btn.disabled = false;
                   finetune_add_btn.disabled = false;

                   document.querySelectorAll('.add-finetune-select-checkpoint-di').forEach(element => {
                        element.addEventListener('click', (e) => {
                            finetune_select_checkpoint_btn.innerText = `${e.currentTarget.innerText}`;
                            finetune_select_checkpoint_btn.dataset.name = e.currentTarget.dataset.name;
                            checkpoint_menu.style.display = 'none';
                        });
                    });

               });
            });

            finetune_add_btn.addEventListener('click', (el) => {
                set_finetune_info_into_state(target.dataset.model, false);
                const spinner = get_spinner();
                el.target.replaceWith(spinner);

                finetune_switch_activate(
                    target.dataset.model,
                    "add",
                    finetune_select_run_btn.dataset.run,
                    finetune_select_checkpoint_btn.dataset.name
                ).then((is_ok) => {
                if (!is_ok) {
                    spinner.replaceWith(el.target);
                    set_finetune_info_into_state(target.dataset.model, true);
                } else {
                    force_render_models_assigned = true;
                }
            });

            });
            document.querySelector("#finetune-select-run-btn-discard").addEventListener('click', (el) => {
                finetune_selectors.remove();
                target.hidden = false;
            });

        });
    });
}

function render_models(models) {
    const models_table = document.querySelector('.table-models tbody');
    models_table.innerHTML = '';
    models.models.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }));
    let models_tree = models.models.reduce((result, item) => {
        const name_parts = item.name.split('/');
        const group_name = name_parts[0];
        if (!result[group_name]) {
          result[group_name] = [];
        }
        result[group_name].push(item);
        return result;
    }, {});
    for (const group_name in models_tree) {
        if (models_tree.hasOwnProperty(group_name)) {
            models_tree[group_name].sort((a, b) => {
            return a.name.localeCompare(b.name);
          });
        }
    }
    for (const [key, value] of Object.entries(models_tree)) {
        console.log(value);
        const row = document.createElement('tr');
        row.setAttribute('data-model',key);
        const model_name = document.createElement("td");
        const model_span = document.createElement('span');
        const has_completion = document.createElement("td");
        const has_finetune = document.createElement("td");
        const has_toolbox = document.createElement("td");
        const has_chat = document.createElement("td");
        model_span.textContent = key;
        model_span.classList.add('model-span');
        model_name.appendChild(model_span);
        row.appendChild(model_name);
        row.appendChild(has_completion);
        row.appendChild(has_finetune);
        row.appendChild(has_toolbox);
        row.appendChild(has_chat);
        models_table.appendChild(row);
        value.forEach(element => {
            const row = document.createElement('tr');
            row.setAttribute('data-model',element.name);
            row.setAttribute('data-parent',key);
            row.classList.add('modelsub-row');
            const model_name = document.createElement("td");
            const has_completion = document.createElement("td");
            const has_finetune = document.createElement("td");
            const has_toolbox = document.createElement("td");
            const has_chat = document.createElement("td");
            model_name.innerHTML = element.name;
            if(element.hasOwnProperty('has_completion')) {
                has_completion.innerHTML = element.has_completion ? '<i class="bi bi-check"></i>' : '';
            }
            if(element.hasOwnProperty('has_finetune')) {
                has_finetune.innerHTML = element.has_finetune ? '<i class="bi bi-check"></i>' : '';
            }
            if(value.hasOwnProperty('has_toolbox')) {
                has_toolbox.innerHTML = element.has_toolbox ? '<i class="bi bi-check"></i>' : '';
            }
            if(element.hasOwnProperty('has_chat')) {
                has_chat.innerHTML = element.has_chat ? '<i class="bi bi-check"></i>' : '';
            }
            row.appendChild(model_name);
            row.appendChild(has_completion);
            row.appendChild(has_finetune);
            row.appendChild(has_toolbox);
            row.appendChild(has_chat);
            models_table.appendChild(row);
            row.addEventListener('click', function(e) {
                const model_name = this.dataset.model;
                models_data.model_assign[model_name] = {
                    gpus_shard: 1
                };
                save_model_assigned();
                const add_model_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('add-model-modal'));
                add_model_modal.hide();
            });
        });
        row.addEventListener('click', function(e) {
            this.classList.toggle('row-active');
            const model_name = this.dataset.model;
            const sub_rows = document.querySelectorAll('[data-parent="'+model_name+'"]');
            sub_rows.forEach(sub_row => {
                sub_row.style.display = sub_row.style.display === 'table-row'? 'none' : 'table-row';
            });
        });
    }
}

function finetune_delete_events() {
    document.querySelectorAll(".btn-remove-run").forEach(element => {
        spawn_finetune_delete_event(element);
    });

}

function get_spinner() {
    const spinner = document.createElement('div');
    const spinner_span = document.createElement('span');
    spinner.className = 'spinner-border';
    spinner.role ='status';
    spinner_span.className ='sr-only';
    spinner.style.scale = '0.5';
    spinner.appendChild(spinner_span);
    return spinner;
}

function spawn_finetune_delete_event(element) {
    let handle_event = (event) => {
        const target = event.currentTarget;
        const spinner = get_spinner();
        set_finetune_info_into_state(target.dataset.model, false);
        target.replaceWith(spinner);

        finetune_switch_activate(
            target.dataset.model,
            "remove",
            target.dataset.run,
            target.dataset.checkpoint
        ).then((is_ok) => {
            if (!is_ok) {
                set_finetune_info_into_state(target.dataset.model, true);
                spinner.replaceWith(target);
            } else {
                force_render_models_assigned = true;
            }
        });
    };

    let clone = element.cloneNode(true);
    // discarding old event handlers
    element.parentNode.replaceChild(clone, element);
    clone.addEventListener("click", handle_event);
}

function format_memory(memory_in_mb, decimalPlaces = 2) {
    return (memory_in_mb / 1024).toFixed(decimalPlaces);
}

export async function init(general_error) {
    let req = await fetch('/tab-model-hosting.html');
    document.querySelector('#model-hosting').innerHTML = await req.text();
    get_gpus();
    get_models();
    const add_model_modal = document.getElementById('add-model-modal');
    add_model_modal.addEventListener('show.bs.modal', function () {
        render_models(models_data);
    });
    const redirect2credentials = document.getElementById('redirect2credentials');
    redirect2credentials.addEventListener('click', function() {
        document.querySelector(`[data-tab=${redirect2credentials.getAttribute('data-tab')}]`).click();
    });
    // const enable_chat_gpt_switch = document.getElementById('enable_chat_gpt');
}

export function tab_switched_here() {
    get_gpus();
    update_finetune_configs_and_runs();
    get_models();
}

export function tab_switched_away() {
}

export function tab_update_each_couple_of_seconds() {
    get_gpus();
    update_finetune_configs_and_runs();
    if (force_render_models_assigned) {
        get_models();
        force_render_models_assigned = false;
    }
}
