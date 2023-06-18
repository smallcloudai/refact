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
        render_models(data);
    });
}
function render_models(models) {
    const models_table = document.querySelector('.table-models tbody');
    models_table.innerHTML = '';
    for(let index in models.models) {
        const row = document.createElement('tr');
        row.setAttribute('datamodel',models.models[index].name);
        const model_name = document.createElement("td");
        const gpu_qty = document.createElement("td");
        const has_chat = document.createElement("td");
        const has_toolbox = document.createElement("td");
        model_name.textContent = models.models[index].name;
        let gpus = 1;
        if(models.model_assign[models.models[index].name] != undefined) {
            row.classList.add('table-success');
            gpus = models.model_assign[models.models[index].name].gpus_max;
        }
        gpu_qty.innerHTML = `<input type="number" step="1" min="0" value="1" class="table-models-gpu form-control">`;
        has_chat.innerHTML = models.models[index].has_chat ? '<i class="bi bi-check"></i>' : '';
        has_toolbox.innerHTML = models.models[index].has_toolbox ? '<i class="bi bi-check"></i>' : '';
        row.appendChild(model_name);
        row.appendChild(gpu_qty);
        row.appendChild(has_chat);
        row.appendChild(has_toolbox);
        models_table.appendChild(row);
        row.addEventListener('click',function(e) {
            document.querySelectorAll('.table-models tbody tr').forEach(function (row) {
                row.classList.remove('table-success');
            });
            this.classList.add('table-success');
            const model_name = e.target.getAttribute('datamodel');
            console.log('model',model_name);
        });
    }
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
}

export function tab_switched_here() {
    get_gpus();
    get_models();
}
