import {general_error} from './error.js';
import {get_finetune_config_and_runs} from './tab-finetune.js';
import {add_finetune_selectors_factory, update_checkpoints_list, set_finetune_info_into_state, finetune_switch_activate} from './utils/tab-model-hosting-utils.js';
import {get_spinner} from "./utils/utils.js";


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

function render_models_assigned(models) {
    const models_info = models_data.models.reduce(function(obj, item) {
      obj[item["name"]] = item;
      return obj;
    }, {});
    const models_table = document.querySelector('.table-assigned-models tbody');

    if (models_table.dataset.hash == CryptoJS.MD5(JSON.stringify(models)) && !force_render_models_assigned) {
        return;
    }
    force_render_models_assigned = false;
    models_table.dataset.hash = CryptoJS.MD5(JSON.stringify(models));

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
            finetune_runs = finetune_configs_and_runs.finetune_runs.filter(
                run => run.model_name === models_info[index].finetune_model
                && run.checkpoints.length !== 0);
            if (models_info[index].finetune_model !== index) {
                finetune_runs = finetune_runs.filter(run => !run.deprecated);
            }
        } else {
            force_render_models_assigned = true;
        }
        finetune_info_factory(index, models_info, finetune_info, finetune_runs, models_data.multiple_loras);

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
}

let toggle_menu_display = (menu) => {
    menu.style.display = (menu.style.display === 'none' || menu.style.display === '') ? 'block' : 'none';
}

function on_add_finetune_btn_click(el, event, models_info) {
    const target = event.currentTarget;

    let finetune_info = document.querySelector(`.model-finetune-info[data-model="${target.dataset.model}"]`);

    let dropdownPanel = document.createElement('div');
    dropdownPanel.classList.add('dropdown-panel-add-finetune')
    let finetune_selectors = add_finetune_selectors_factory(finetune_configs_and_runs, models_info, target.dataset.model);

    dropdownPanel.appendChild(finetune_selectors);

    for (let old_panel of document.querySelectorAll('.dropdown-panel-add-finetune')) {
        old_panel.remove();
    }
    finetune_info.appendChild(dropdownPanel);
    dropdownPanel.style.display = 'block';

    let finetune_add_btn = document.querySelector("#finetune-select-run-btn-add");
    finetune_add_btn.disabled = true;

    let finetune_select_run_btn = document.getElementById('add-finetune-select-run-btn');

    let run_menu = document.getElementById('add-finetune-select-run-menu');

    let finetune_select_checkpoint_btn = document.getElementById('add-finetune-select-checkpoint-btn');
    let checkpoint_menu = document.getElementById('add-finetune-select-checkpoint-menu');


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
        // if (!dropdownPanel.contains(event.target) && event.currentTarget !== target) {
        //     dropdownPanel.style.display = 'none';
        // }
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
    let discard_btn = document.querySelector("#finetune-select-run-btn-discard");

    finetune_add_btn.addEventListener('click', (el) => {
        discard_btn.disabled = true;
        const spinner = get_spinner();
        el.target.replaceWith(spinner);

        finetune_switch_activate(
            target.dataset.model,
            "add",
            finetune_select_run_btn.dataset.run,
            finetune_select_checkpoint_btn.dataset.name
        ).then((is_ok) => {
        if (!is_ok) {
            discard_btn.disabled = false;
            spinner.replaceWith(el.target);
        } else {
            force_render_models_assigned = true;
        }
    });

    });
    document.querySelector("#finetune-select-run-btn-discard").addEventListener('click', (el) => {
        finetune_selectors.remove();
        dropdownPanel.style.display = 'none';
        target.disabled = false;
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

function finetune_info_factory(index, models_info, finetune_info, finetune_runs, multiple_loras) {
    let enabled_finetunes = [];

    if (models_info[index].hasOwnProperty('finetune_info') && models_info[index].finetune_info) {
        for (let run of models_info[index].finetune_info) {
            let enabled_finetune = document.createElement("div");
            enabled_finetune.dataset.run = run.run_id;
            enabled_finetune.dataset.checkpoint = run.checkpoint;
            enabled_finetune_factory(enabled_finetune, index);
            enabled_finetunes.push(enabled_finetune);
        }
    }

    let tech_msg = document.createElement("div");
    tech_msg.classList.add("tech-msg");

    if (!models_info[index].has_finetune) {
        tech_msg.innerText = "Finetune not supported";
        finetune_info.appendChild(tech_msg);
    } else if (finetune_runs.length == 0) {
        tech_msg.innerText = "no finetunes available";
        finetune_info.appendChild(tech_msg);
    } else {
        let finetune_info_children = document.createElement("div");
        for (let child of enabled_finetunes) {
            finetune_info_children.appendChild(child);
        }
        finetune_info.appendChild(finetune_info_children);

        const selected_runs = models_info[index].finetune_info.map(run => run.run_id);
        const not_selected_runs = finetune_runs.filter(run => !selected_runs.includes(run.run_id));
        if (not_selected_runs.length > 0 && (selected_runs.length === 0 || multiple_loras)) {
            let add_finetune_btn = document.createElement("button");
            add_finetune_btn.classList = "btn btn-sm btn-outline-primary mt-1 add-finetune-btn";
            add_finetune_btn.dataset.model = index;
            add_finetune_btn.innerText = 'Add Finetune';
            finetune_info.appendChild(add_finetune_btn);

            add_finetune_btn.addEventListener('click', (event) => {
                on_add_finetune_btn_click(add_finetune_btn, event, models_info);
            });
        }
    }
}

function enabled_finetune_factory(enabled_finetune, model) {
    let outer_div = document.createElement('div');
    let inner_div = document.createElement('div');
    let upper_row_div = document.createElement('div');
    let run_div = document.createElement('div');
    let button_div = document.createElement('div');
    let button = document.createElement('button');
    let checkpoint_div = document.createElement('div');

    outer_div.classList.add('model-finetune-item');
    inner_div.classList.add('model-finetune-item-inner');
    upper_row_div.classList.add('model-finetune-item-upper-row');
    run_div.classList.add('model-finetune-item-run');
    button_div.style.marginLeft = 'auto';
    button.classList.add('btn-remove-run');
    checkpoint_div.classList.add('model-finetune-item-checkpoint');

    outer_div.dataset.run = enabled_finetune.dataset.run;
    run_div.textContent = enabled_finetune.dataset.run;
    button.dataset.run = enabled_finetune.dataset.run;
    button.dataset.checkpoint = enabled_finetune.dataset.checkpoint;
    button.dataset.model = model;
    button.textContent = 'x';
    checkpoint_div.textContent = 'Checkpoint: ' + enabled_finetune.dataset.checkpoint;

    upper_row_div.appendChild(run_div);
    button_div.appendChild(button);
    upper_row_div.appendChild(button_div);
    inner_div.appendChild(upper_row_div);
    inner_div.appendChild(checkpoint_div);
    outer_div.appendChild(inner_div);

    enabled_finetune.innerHTML = '';
    enabled_finetune.appendChild(outer_div);

    button.addEventListener('click', () => {
        const spinner = get_spinner();
        spinner.style.padding = '0';
        spinner.style.margin = '0';
        spinner.style.scale = '0.5';
        spinner.style.position = 'relative';

        set_finetune_info_into_state(button.dataset.model, false);
        button.replaceWith(spinner);

        finetune_switch_activate(
            button.dataset.model,
            "remove",
            button.dataset.run,
            button.dataset.checkpoint
        ).then((is_ok) => {
            if (!is_ok) {
                set_finetune_info_into_state(button.dataset.model, true);
                spinner.replaceWith(button);
            } else {
                force_render_models_assigned = true;
            }
        });
    });
}

function format_memory(memory_in_mb, decimalPlaces = 2) {
    return (memory_in_mb / 1024).toFixed(decimalPlaces);
}

export async function init(general_error) {
    let req = await fetch('/tab-model-hosting.html');
    document.querySelector('#model-hosting').innerHTML = await req.text();
    get_gpus();
    update_finetune_configs_and_runs();
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
    }
}
