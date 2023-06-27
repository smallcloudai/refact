let logstream_reader = null;
let logstream_runid = null;
let downloaded_data = null;
let blue_lora = "";
let loras_switch_off = null;
let loras_switch_latest = null;
let loras_switch_specific = null;
let loras_switch_no_reaction = false;
// let checkpoint_name = "best";
// let selected_model = ""; // we don't have model choice, empty for now

function finetune_data() {
    fetch("/tab-finetune-config-and-runs")
        .then(function (response) {
            return response.json();
        })
        .then(function (data) {
            console.log('tab-finetune-config-and-runs',data);
            render_finetune_settings(data);
            downloaded_data = data;
            render_lora_switch();
            render_runs();
        });
}

function render_finetune_settings(data = {}) {
    if (data.config.auto_delete_n_runs) {
        document.querySelector('.store-input').value = data.config.auto_delete_n_runs;
    }
    if (data.config.limit_training_time_minutes) {
        const radio_limit_time = document.querySelector(`input[name="limit_training_time_minutes"][value="${data.config.limit_training_time_minutes}"]`);
        if (radio_limit_time) {
            radio_limit_time.checked = true;
        }
    }
    if (data.config.run_at_night) {
        document.querySelector('#night_run').checked = true;
    }
    if (data.config.run_at_night_time) {
        const selectElement = document.querySelector('.night-time');
        const optionToSelect = selectElement.querySelector(`option[value="${data.config.run_at_night_time}"]`);
        if (optionToSelect) {
            optionToSelect.selected = true;
        }
    }
}

function delete_run(run_id) {
    fetch(`/tab-finetune-remove/${run_id}`)
    .then(response => {
        if (!response.ok) {
            return response.json()
            .then(error => {
                throw new Error(error.message);
            });
        }
    })
}

function render_runs() {
    let data = downloaded_data;
    let is_working = false;
    document.querySelector('.run-table').innerHTML = '';
    if(data.finetune_runs.length === 0) {
        document.querySelector('.table-types').style.display = 'none';
        return;
    }
    document.querySelector('.table-types').style.display = 'table';
    data.finetune_runs.forEach(element => {
        const row = document.createElement('tr');
        row.style.whiteSpace = 'nowrap';
        const run_name = document.createElement("td");
        const run_status = document.createElement("td");
        const run_minutes = document.createElement("td");
        const run_steps = document.createElement("td");
        const run_delete = document.createElement("td");

        run_name.innerText = element.run_id;
        let status_color;
        switch (element.status) {
            case 'unknown':
                status_color = `text-bg-warning`;
                break;
            case 'starting':
                status_color = `text-bg-secondary`;
                break;
            case 'working':
                status_color = `text-bg-secondary`;
                break;
            case 'completed':
            case 'finished':
                status_color = `text-bg-success`;
                break;
            case 'failed':
                status_color = `text-bg-danger`;
                break;
            default:
                status_color = `text-bg-info`;
                break;
        }

        row.dataset.run = element.run_id;
        const local_is_working = element.status === 'working';
        if (local_is_working) {
            is_working = true;
            if (!blue_lora) {
                blue_lora = element.run_id;
            }
            run_status.innerHTML = `<span class="badge rounded-pill ${status_color}"><div class="finetune-spinner spinner-border spinner-border-sm" role="status"></div>${element.status}</span>`;
        } else {
            run_status.innerHTML = `<span class="badge rounded-pill ${status_color}">${element.status}</span>`;
        }
        run_minutes.innerHTML = element.worked_minutes;
        run_steps.innerHTML = element.worked_steps;
        const disabled = local_is_working ? "disabled" : ""
        run_delete.innerHTML = `<button class="btn btn-danger btn-sm" ${disabled}"><i class="bi bi-trash3-fill"></i></button>`;
        row.appendChild(run_name);
        row.appendChild(run_status);
        row.appendChild(run_minutes);
        row.appendChild(run_steps);
        row.appendChild(run_delete);
        run_delete.addEventListener('click', () => {
            delete_run(element.run_id);
        })

        document.querySelector('.run-table').appendChild(row);
        if (blue_lora == element.run_id) {
            row.classList.add('table-success');
            const timestamp = new Date().getTime();
            const gfx = document.querySelector('.fine-gfx');
            gfx.src = `/tab-finetune-progress-svg/${element.run_id}?t=${timestamp}`;
            start_log_stream(element.run_id);
            const log_link = document.querySelector('.log-link');
            if(log_link.classList.contains('d-none')) {
                log_link.classList.remove('d-none');
            }
            log_link.href = `/tab-finetune-log/${element.run_id}`;
        }
    });
    const rows = document.querySelectorAll('.run-table tr');
    rows.forEach(function (row) {
        row.addEventListener('click', function (event) {
            event.stopPropagation();
            const run_id = this.dataset.run;
            blue_lora = run_id;
            render_runs();
            render_checkpoints(find_checkpoints_by_run(run_id));
        });
    });
    const start_finetune_button = document.querySelector('.tab-finetune-run-now');
    if(is_working) {
        start_finetune_button.innerHTML = '<div class="upload-spinner spinner-border spinner-border-sm" role="status"></div>' + 'Stop';
    } else {
        start_finetune_button.textContent = 'Start Now';
    }
    start_finetune_button.disabled = ![undefined, 'interrupted', 'finished', 'error'].includes(data.filtering_status)
}

const find_checkpoints_by_run = (run_id) => {
    const finetune_run = downloaded_data.finetune_runs.find((run) => run.run_id === run_id);
    if (finetune_run) {
      return finetune_run.checkpoints;
    } else {
      return null;
    }
};

function render_lora_switch() {
    let mode = downloaded_data.active ? downloaded_data.active.lora_mode : "latest-best";
    loras_switch_no_reaction = true; // avoid infinite loop when setting .checked
    if (mode === 'off') {
        loras_switch_off.checked = true;
    } else if (mode === 'latest-best') {
        loras_switch_latest.checked = true;
    } else if (mode === 'specific') {
        loras_switch_specific.checked = true
    }
    loras_switch_no_reaction = false;
    let lora_switch_run_id = document.querySelector('#lora-switch-run-id');
    let lora_switch_checkpoint = document.querySelector('#lora-switch-checkpoint');
    if (mode === 'specific') {
        lora_switch_run_id.style.display = 'block';
        lora_switch_checkpoint.style.display = 'block';
        lora_switch_run_id.style.opacity = 1;
        lora_switch_checkpoint.style.opacity = 1;
        lora_switch_run_id.innerHTML = `<b>Run:</b> ${downloaded_data.active.specific_lora_run_id}`;
        lora_switch_checkpoint.innerHTML = `<b>Checkpoint:</b> ${downloaded_data.active.specific_checkpoint}`;
    } else if (mode == 'latest-best') {
        lora_switch_run_id.style.display = 'block';
        lora_switch_checkpoint.style.display = 'block';
        lora_switch_run_id.style.opacity = 0.5;
        lora_switch_checkpoint.style.opacity = 0.5;
        lora_switch_run_id.innerHTML = `<b>Run:</b> ${downloaded_data.finetune_latest_best.latest_run_id}`;
        lora_switch_checkpoint.innerHTML = `<b>Checkpoint:</b> ${downloaded_data.finetune_latest_best.best_checkpoint_id}`;
    } else {
        lora_switch_run_id.style.display = 'none';
        lora_switch_checkpoint.style.display = 'none';
        lora_switch_run_id.innerHTML = `<b>Run:</b> ${downloaded_data.active.specific_lora_run_id}`;
        lora_switch_checkpoint.innerHTML = `<b>Checkpoint:</b> ${downloaded_data.active.specific_checkpoint}`;
    }
}

function render_checkpoints(data = []) {
    const checkpoints = document.querySelector('.table-checkpoints');
    checkpoints.innerHTML = '';
    if (data.length > 0) {
        data.forEach(element => {
            const row = document.createElement('tr');
            const cell = document.createElement('td');
            cell.textContent = `${element.checkpoint_name}`;
            cell.dataset.checkpoint = element.checkpoint_name;
            if(cell.dataset.checkpoint === downloaded_data.active.specific_checkpoint) {
                row.classList.add('table-success');
            }
            row.appendChild(cell);
            checkpoints.appendChild(row);
            row.addEventListener('click', (event) => {
                if(!row.classList.contains('table-success')) {
                    document.querySelector('.table-checkpoints .table-success').classList.remove('table-success');
                    row.classList.add('table-success');
                }
                finetune_switch_activate("specific", blue_lora, cell.dataset.checkpoint);
            });
        });
    }
}

function loras_switch_clicked() {
    if (loras_switch_no_reaction)
        return;
    if (loras_switch_off.checked === true) {
        finetune_switch_activate("off");
    } else if (loras_switch_latest.checked === true) {
        finetune_switch_activate("latest-best");
    } else if (loras_switch_specific.checked === true) {
        finetune_switch_activate("specific");
    }
}

function finetune_switch_activate(lora_mode, run_id, checkpoint) {
    let send_this = {
        "model": "",
        "lora_mode": lora_mode,
        "specific_lora_run_id": run_id ? run_id : downloaded_data.active.specific_lora_run_id,
        "specific_checkpoint": checkpoint ? checkpoint : downloaded_data.active.specific_checkpoint,
    }
    console.log(send_this);
    fetch("/tab-finetune-activate", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(send_this)
    })
    .then(function (response) {
        finetune_data();
    });
}

function render_time_dropdown() {
    const selectElement = document.querySelector('.night-time');
    for (let hour = 0; hour < 24; hour++) {
        const option = document.createElement("option");
        const formattedHour = hour.toString().padStart(2, "0");

        option.value = formattedHour + ":00";
        option.text = formattedHour + ":00";
        selectElement.appendChild(option);
    }
}
const finetune_inputs = document.querySelectorAll('.fine-tune-input');
for (let i = 0; i < finetune_inputs.length; i++) {
    finetune_inputs[i].addEventListener('change', function () {
        save_finetune_schedule();
    });
}
function save_finetune_schedule() {
    const data = {
        "limit_training_time_minutes": document.querySelector('input[name="limit_training_time_minutes"]:checked').value,
        "run_at_night": document.querySelector('#night_run').checked,
        "run_at_night_time": document.querySelector('.night-time').value,
        "auto_delete_n_runs": document.querySelector('.store-input').value,
    }
    console.log('save_finetune_settings', data);
    fetch("/tab-finetune-config-save", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(data)
    })
    .then(function (response) {
        console.log(response);
        finetune_data();
    });
}

function start_log_stream(run_id) {
    if (run_id === logstream_runid) {
        return;
    }
    if (logstream_reader) {
        logstream_reader.cancel();
    }

    const log_div = document.querySelector('.tab-upload-finetune-logs');
    log_div.textContent = '';

    const streamTextFile = async () => {
        const decoder = new TextDecoder();
        const response = await fetch(`/tab-finetune-log/${run_id}`);
        const reader = response.body.getReader();
        logstream_reader = reader;
        logstream_runid = run_id;

        const processResult = ({ done, value }) => {
            if (done) {
                console.log('Streaming complete');
                return;
            }

            const chunk = decoder.decode(value);


            const isAtBottom = log_div.scrollTop >= (log_div.scrollHeight - log_div.offsetHeight);

            log_div.textContent += chunk;

            if (isAtBottom) {
                log_div.scrollTop = log_div.scrollHeight;
            }
            const timestamp = new Date().getTime();
            const gfx = document.querySelector('.fine-gfx');
            gfx.src = `/tab-finetune-progress-svg/${run_id}?t=${timestamp}`;
            return reader.read().then(processResult);
        };

        return reader.read().then(processResult);
    };

    streamTextFile()
        .catch(error => {
            console.log('Error:', error);
        });
}

const log_container = document.querySelector('.log-container');
function handle_auto_scroll() {
    if (log_container.scrollHeight - log_container.scrollTop === log_container.clientHeight) {
        log_container.scrollTop = log_container.scrollHeight;
    }
}
log_container.addEventListener('scroll', handle_auto_scroll);

function get_finetune_settings(defaults = false) {
    fetch("/tab-finetune-training-get")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        console.log('tab-finetune-training-get',data);
        let settings_data = null;
        if(Object.keys(data.user_config).length > 0 && !defaults) {
            settings_data = data.user_config;
        } else {
            settings_data = data.defaults;
        }
        document.querySelector('#finetune-tab-settings-modal #limit_time_seconds').value = settings_data.limit_time_seconds;
        document.querySelector('#finetune-tab-settings-modal #max_iterations').value = settings_data.max_iterations;
        document.querySelector('#finetune-tab-settings-modal #max_epoch').value = settings_data.max_epoch;
        document.querySelector('#finetune-tab-settings-modal #warmup_num_steps').value = settings_data.warmup_num_steps;
        document.querySelector('#finetune-tab-settings-modal #batch_size').value = settings_data.batch_size;
        document.querySelector('#finetune-tab-settings-modal #lr').value = settings_data.lr;
        document.querySelector('#finetune-tab-settings-modal #lr_decay_steps').value = settings_data.lr_decay_steps;
        document.querySelector('#finetune-tab-settings-modal #lora_r').value = settings_data.lora_r;
        document.querySelector('#finetune-tab-settings-modal #lora_init_scale').value = settings_data.lora_init_scale;
        document.querySelector('#finetune-tab-settings-modal #lora_dropout').value = settings_data.lora_dropout;
        document.querySelector('#finetune-tab-settings-modal #weight_decay').value = settings_data.weight_decay;
        const low_gpu_mem_mode = settings_data.low_gpu_mem_mode;
        if(low_gpu_mem_mode ) {
            document.querySelector('#finetune-tab-settings-modal #low_gpu_mem_mode_finetune').checked = true;
        } else {
            document.querySelector('#finetune-tab-settings-modal #low_gpu_mem_mode_finetune').checked = false;
        }
    });
}

function save_finetune_settings() {
    console.log('save_finetune_settings');
    let low_gpu = false;
    if (document.querySelector('#finetune-tab-settings-modal #low_gpu_mem_mode_finetune').checked) {
        low_gpu = true;
    }
    fetch("/tab-finetune-smart-filter-setup", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            limit_time_seconds: document.querySelector('#finetune-tab-settings-modal #limit_time_seconds').value,
            max_iterations: document.querySelector('#finetune-tab-settings-modal #max_iterations').value,
            max_epoch: document.querySelector('#finetune-tab-settings-modal #max_epoch').value,
            warmup_num_steps: document.querySelector('#finetune-tab-settings-modal #warmup_num_steps').value,
            batch_size: document.querySelector('#finetune-tab-settings-modal #batch_size').value,
            lr: document.querySelector('#finetune-tab-settings-modal #lr').value,
            lr_decay_steps: document.querySelector('#finetune-tab-settings-modal #lr_decay_steps').value,
            lora_r: document.querySelector('#finetune-tab-settings-modal #lora_r').value,
            lora_init_scale: document.querySelector('#finetune-tab-settings-modal #lora_init_scale').value,
            lora_dropout: document.querySelector('#finetune-tab-settings-modal #lora_dropout').value,
            weight_decay: document.querySelector('#finetune-tab-settings-modal #weight_decay').value,
            low_gpu_mem_mode: low_gpu
        })
    })
    .then(function(response) {
        if(response.ok) {
            get_finetune_settings();
        }
    });
}

export function init() {
    const start_finetune_button = document.querySelector('.tab-finetune-run-now');
    start_finetune_button.addEventListener('click', function () {
        fetch("/tab-finetune-run-now")
            .then(function (response) {
                finetune_data();
            })
    });
    const loras = document.querySelectorAll('.lora-switch');
    loras.forEach(element => {
        if (element.value === 'off')
            loras_switch_off = element;
        if (element.value === 'latest')
            loras_switch_latest = element;
        if (element.value === 'specific')
            loras_switch_specific = element;
    });
    loras_switch_off.addEventListener('change', loras_switch_clicked);
    loras_switch_latest.addEventListener('change', loras_switch_clicked);
    loras_switch_specific.addEventListener('change', loras_switch_clicked);
    const loras_table = document.querySelector('.run-table-wrapper');
    loras_table.scrollTop = loras_table.scrollHeight;

    const finetune_modal = document.getElementById('finetune-tab-settings-modal');
    finetune_modal.addEventListener('show.bs.modal', function () {
        get_finetune_settings();
    });

    const finetune_submit = document.querySelector('.finetune-tab-settings-submit');
    finetune_submit.addEventListener('click', function() {
        save_finetune_settings();
    });

    const finetune_modal_defaults = document.querySelector('.finetune-tab-settings-default');
    finetune_modal_defaults.addEventListener('click', function() {
        get_finetune_settings(true);
    });
}

export function tab_switched_here() {
    finetune_data();
    render_time_dropdown();
}
