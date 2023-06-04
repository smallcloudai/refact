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
    const gpus_table = document.querySelector('.table-gpus tbody');
    gpus_table.innerHTML = '';
    gpus.forEach(element => {
        const row = document.createElement('tr');
        row.setAttribute('gpu',element.id);
        const gpu_name = document.createElement("td");
        const gpu_mem = document.createElement("td");
        const gpu_temp = document.createElement("td");
        
        const used_gb = Math.round(element.mem_used_mb / 1024);
        const total_gb = Math.round(element.mem_total_mb /1024);
        const used_mem = Math.round(element.mem_used_mb / (element.mem_total_mb / 100));
        gpu_name.innerHTML = `<i class="table-gpu-card bi bi-gpu-card"></i>` + element.name;
        gpu_mem.innerHTML = `<div class="table-gpu-mem"><span style="width: ${used_mem}%"></span></div>${used_gb}/${total_gb} GB`;
        gpu_temp.textContent = element.temp_celsius + '°C';
        row.appendChild(gpu_name);
        row.appendChild(gpu_mem);
        row.appendChild(gpu_temp);
        gpus_table.appendChild(row);
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
            document.querySelectorAll('.table-models tbody tr').forEach(function (row) {
                row.classList.remove('table-primary');
            });
            this.classList.add('table-primary');
            const model_name = e.target.getAttribute('datamodel');
            console.log('model',model_name);
        });
    }
}

export function init() {
    get_gpus();
    get_models();
}