import { general_error } from './error.js';
let gpus_popup = false;
let models_data = null;

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
        gpu_temp.innerHTML = `<b>Temp</b>` + element.temp_celsius + '°C';
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

function get_models()
{
    fetch("/tab-host-models-get")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        models_data = data;
        render_models_assigned(data.model_assign);
        const enable_chat_gpt_switch = document.getElementById('enable_chat_gpt');
        enable_chat_gpt_switch.removeEventListener('change', save_model_assigned);
        enable_chat_gpt_switch.checked = models_data['openai_api_enable'];
        enable_chat_gpt_switch.addEventListener('change', save_model_assigned);
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
        console.log('tab-host-models-get',error);
        general_error(error);
    });
}

function save_model_assigned() {
    let openai_enable = document.querySelector('#enable_chat_gpt');
    const data = {
        model_assign: {
            ...models_data.model_assign,
        },
        completion: models_data.completion ? models_data.completion : "",
        openai_api_enable: openai_enable.checked
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
    models_table.innerHTML = '';
    for(let index in models) {
        const row = document.createElement('tr');
        row.setAttribute('data-model',index);
        const model_name = document.createElement("td");
        const completion = document.createElement("td");
        const finetune_info = document.createElement("td");
        const select_gpus = document.createElement("td");
        const gpus_share = document.createElement("td");
        const del = document.createElement("td");

        model_name.textContent = index;

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

        if (models_info[index].hasOwnProperty('finetune_info') && models_info[index].finetune_info) {
            finetune_info.innerHTML = `
            <style>
              table {
                border-collapse: separate;
                border-spacing: 10px;
              }
            </style>
            <table>
                <tr>
                    <td>Run: </td>
                    <td>${models_info[index].finetune_info.run}</td>
                </tr>
                <tr>
                    <td>Checkpoint: </td>
                    <td>${models_info[index].finetune_info.checkpoint}</td>
                </tr>
            </table>
            `;
        }

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
        del_button.classList.add('model-remove','btn','btn-danger');
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

function render_models(models) {
    const models_table = document.querySelector('.table-models tbody');
    models_table.innerHTML = '';
    models.models.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }));
    for(let index in models.models) {
        console.log('xxxx',models.models);
        const row = document.createElement('tr');
        row.setAttribute('data-model',models.models[index].name);
        const model_name = document.createElement("td");
        const has_completion = document.createElement("td");
        const has_finetune = document.createElement("td");
        const has_toolbox = document.createElement("td");
        const has_chat = document.createElement("td");
        model_name.textContent = models.models[index].name;
        if(models.models[index].hasOwnProperty('has_completion')) {
            has_completion.innerHTML = models.models[index].has_completion ? '<i class="bi bi-check"></i>' : '';
        }
        if(models.models[index].hasOwnProperty('has_finetune')) {
            has_finetune.innerHTML = models.models[index].has_finetune ? '<i class="bi bi-check"></i>' : '';
        }
        if(models.models[index].hasOwnProperty('has_toolbox')) {
            has_toolbox.innerHTML = models.models[index].has_toolbox ? '<i class="bi bi-check"></i>' : '';
        }
        if(models.models[index].hasOwnProperty('has_chat')) {
            has_chat.innerHTML = models.models[index].has_chat ? '<i class="bi bi-check"></i>' : '';
        }
        row.appendChild(model_name);
        row.appendChild(has_completion);
        row.appendChild(has_finetune);
        row.appendChild(has_toolbox);
        row.appendChild(has_chat);
        models_table.appendChild(row);
        row.addEventListener('click', function(e) {
            models_data.model_assign[models.models[index].name] = {
                gpus_shard: 1
            };
            save_model_assigned();
            const add_model_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('add-model-modal'));
            add_model_modal.hide();
        });
    }
}

function format_memory(memory_in_mb, decimalPlaces = 2) {
    const memory_in_gb = (memory_in_mb / 1024).toFixed(decimalPlaces);
    return memory_in_gb;
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
    get_models();
}

export function tab_switched_away() {
}

export function tab_update_each_couple_of_seconds() {
    get_gpus();
}
