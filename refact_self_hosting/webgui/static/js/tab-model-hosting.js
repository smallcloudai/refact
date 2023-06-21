import {update_integrations} from "./tab-credentials-settings.js";

let gpus_popup = false;
let models_data = null;
let models_gpus_change = false;
function get_gpus() {
    fetch("/tab-host-have-gpus")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        console.log('gpus',data);
        render_gpus(data);
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
        const gpu_command = document.createElement("div");
        gpu_command.classList.add('gpus-command');
        const gpu_status = document.createElement("div");
        gpu_status.classList.add('gpus-status');

        const used_gb = format_memory(element.mem_used_mb);
        const total_gb = format_memory(element.mem_total_mb);
        const used_mem = Math.round(element.mem_used_mb / (element.mem_total_mb / 100));
        gpu_name.innerHTML = element.name;
        gpu_mem.innerHTML = `<b>Mem</b><div class="gpus-mem-wrap"><div class="gpus-mem-bar"><span style="width: ${used_mem}%"></span></div>${used_gb}/${total_gb} GB</div>`;
        gpu_temp.innerHTML = `<b>Temp</b>` + element.temp_celsius + '°C';
        gpu_command.innerHTML = `<span class="gpus-current-status">${element.status}</span>`;
        gpu_status.innerHTML += `<div><b>Command</b>${element.command}</div>`;
        gpu_status.innerHTML += `<div><b>Status</b>${element.status}</div>`;
        gpu_command.appendChild(gpu_status);
        gpu_command.addEventListener('mouseover',function(e) {
            gpus_popup = true;
            this.querySelector('.gpus-status').classList.add('gpus-status-visible');
        });
        gpu_command.addEventListener('mouseout',function(e) {
            gpus_popup = false;
            this.querySelector('.gpus-status').classList.remove('gpus-status-visible');
        });
        if(!element.status || element.status === '') {
            gpu_command.classList.add('gpus-status-invisible');
        }
        row.appendChild(gpu_image);
        gpu_wrapper.appendChild(gpu_name);
        gpu_wrapper.appendChild(gpu_mem);
        gpu_wrapper.appendChild(gpu_temp);
        gpu_wrapper.appendChild(gpu_command);
        row.appendChild(gpu_wrapper);
        gpus_list.appendChild(row);
        
    });
}
function get_models() {
    fetch("/tab-host-models-get")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        console.log('models',data);
        models_data = data;
        render_models_assigned(data.model_assign);
    });
}
function render_models_assigned(models) {
    if(models_gpus_change) {
        return;
    }
    const models_table = document.querySelector('.table-assigned-models tbody');
    models_table.innerHTML = '';
    for(let index in models) {
        const row = document.createElement('tr');
        row.setAttribute('datamodel',index);
        const model_name = document.createElement("td");
        const gpus = document.createElement("td");
        const gpus_input = document.createElement("input");
        const del = document.createElement("td");
        const del_button = document.createElement("button");
        model_name.textContent = index;
        gpus_input.classList.add('model-gpus','form-control');
        gpus_input.setAttribute('model',index);
        gpus_input.setAttribute('min',1);
        gpus_input.setAttribute('max',8);
        gpus_input.setAttribute('step',1);
        gpus_input.setAttribute('type','number');
        gpus_input.value = models[index].gpus_min;
        gpus_input.addEventListener('change', function() {
            models[index].gpus_min = this.value;
            let openai_enable = document.querySelector('#enable_chat_gpt');
            const data = {
                model_assign: {
                    ...models,
                },
                openai_enable: openai_enable.checked
            };
            fetch("/tab-host-models-assign", {
                method: "POST",
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(data)
            })
            .then(function (response) {
                models_gpus_change = false;
                console.log('response',response);
            });
        });
        gpus_input.addEventListener('blur', function() {
            models_gpus_change = true;
        });
        
        gpus.appendChild(gpus_input);
        del_button.innerHTML = `<i class="bi bi-trash3-fill"></i>`;
        del_button.dataset.model = index;
        del_button.addEventListener('click', function() {
            delete_model(index);
        });
        del_button.classList.add('model-remove','btn','btn-danger');
        del.appendChild(del_button);
        // del.innerHTML = `<button type="button" data-model="${index}" class="btn btn-danger model-remove"><i class="bi bi-trash3-fill"></i></button>`;
        row.appendChild(model_name);
        row.appendChild(gpus);
        row.appendChild(del);
        models_table.appendChild(row);
    }
}
function render_models(models) {
    const models_table = document.querySelector('.table-models tbody');
    models_table.innerHTML = '';
    for(let index in models.models) {
        const row = document.createElement('tr');
        row.setAttribute('datamodel',models.models[index].name);
        const model_name = document.createElement("td");
        const has_chat = document.createElement("td");
        const has_toolbox = document.createElement("td");
        model_name.textContent = models.models[index].name;
        has_chat.innerHTML = models.models[index].has_chat ? '<i class="bi bi-check"></i>' : '';
        has_toolbox.innerHTML = models.models[index].has_toolbox ? '<i class="bi bi-check"></i>' : '';
        row.appendChild(model_name);
        row.appendChild(has_chat);
        row.appendChild(has_toolbox);
        models_table.appendChild(row);
        row.addEventListener('click',function(e) {
            let openai_enable = document.querySelector('#enable_chat_gpt');
            const data = {
                model_assign: {
                    ...models_data.model_assign,
                },
                openai_enable: openai_enable.checked
            };
            data.model_assign[models.models[index].name] = {
                gpus_min: 1
            };
            fetch("/tab-host-models-assign", {
                method: "POST",
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(data)
            })
            .then(function (response) {
                const add_model_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('add-model-modal'));
                add_model_modal.hide();
                console.log('response',response);
            });
        });
    }
}

function delete_model(model_name) {
    let data = models_data.model_assign;
    let openai_enable = document.querySelector('#enable_chat_gpt');
    delete data[model_name];
    const updated_data = {
        model_assign: {
           ...data,
        },
        openai_enable: openai_enable.checked
    };
    fetch("/tab-host-models-assign", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(updated_data)
    })
    .then(function (response) {
        console.log('response',response);
    });
}

function format_memory(memory_in_mb, decimalPlaces = 2) {
    const memory_in_gb = (memory_in_mb / 1024).toFixed(decimalPlaces);
    return memory_in_gb;
}

export function init() {
    get_gpus();
    get_models();
    const chat_gpt_switch = document.querySelector('#enable_chat_gpt');
    const chat_gpt_api_input = document.querySelector('.chat-gpt-key');

    chat_gpt_switch.addEventListener('change', function() {
      if (this.checked) {
        chat_gpt_api_input.classList.remove('d-none');
      } else {
        chat_gpt_api_input.classList.add('d-none');
      }
    });
    const add_model_modal = document.getElementById('add-model-modal');
    add_model_modal.addEventListener('show.bs.modal', function () {
        render_models(models_data);
    });
}

export function tab_switched_here() {
    get_gpus();
    get_models();
    update_integrations();
}
