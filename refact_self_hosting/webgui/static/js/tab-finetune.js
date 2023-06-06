let logstream_reader = null;
let logstream_runid = null;
let downloaded_data = null;
let checkpoint_name = "best";
let selected_lora = "latest";
let selected_model = "";

function finetune_data() {
    fetch("/tab-finetune-config-and-runs")
        .then(function (response) {
            return response.json();
        })
        .then(function (data) {
            console.log('config-and-runs',data);
            render_finetune_settings(data);
            render_checkpoints(data);
            downloaded_data = data;
            render_loras();
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

function render_runs() {
    let data = downloaded_data;
    document.querySelector('.run-table').innerHTML = '';
    data.finetune_runs.forEach(element => {
        const row = document.createElement('tr');
        const run_name = document.createElement("td");
        const run_status = document.createElement("td");
        const run_minutes = document.createElement("td");
        const run_steps = document.createElement("td");

        run_name.innerHTML = element.run_id;
        let status_color;
        switch (element.status) {
            case 'unknown':
                status_color = `bg-warning text-dark`;
                break;
            case 'starting':
                status_color = `bg-success`;
                break;
            case 'working':
                status_color = `bg-secondary`;
                break;
            case 'completed':
                status_color = `bg-primary`;
                break;
            case 'failed':
                status_color = `bg-danger`;
                break;
        }

        row.dataset.run = element.run_id;
        if (element.status === 'working') {
            if (!selected_lora) {
                selected_lora = element.run_id;
            }
            run_status.innerHTML = `<span class="badge rounded-pill ${status_color}"><div class="finetune-spinner spinner-border spinner-border-sm" role="status"></div>${element.status}</span>`;
        } else {
            run_status.innerHTML = `<span class="badge rounded-pill ${status_color}">${element.status}</span>`;
        }
        run_minutes.innerHTML = element.worked_minutes;
        run_steps.innerHTML = element.worked_steps;
        row.appendChild(run_name);
        row.appendChild(run_status);
        row.appendChild(run_minutes);
        row.appendChild(run_steps);
        document.querySelector('.run-table').appendChild(row);
        if (selected_lora == element.run_id) {
            row.classList.add('table-primary');
            document.querySelector('.fine-gfx').src = `/tab-finetune-progress-svg/${element.run_id}`;
            console.log(`/tab-finetune-progress-svg/${element.run_id}`);
            start_log_stream(element.run_id);
        }
    });
    const rows = document.querySelectorAll('.run-table tr');
    rows.forEach(function (row) {
        row.addEventListener('click', function (event) {
            event.stopPropagation();
            const run_id = this.dataset.run;
            selected_lora = run_id;
            render_runs();
        });
    });
}

function render_loras() {
    const loras = document.querySelector('.lora-input');
    loras.forEach(element => {
        element.addEventListener('change', function () {
            if(element.checked === true) {
                selected_lora = element.value;
            }
            finetune_activate();
        });
    });
}

function render_checkpoints(data = {}) {
    const checkpoints = document.querySelector('.checkpoints');
    checkpoints.innerHTML = '';
    if (data.checkpoints.length === 0) {
        data.checkpoints.forEach(element => {
            const row = document.createElement('div');
            row.classList.add('checkpoints-row');
            row.dataset.checkpoint = element.checkpoint_name;
            row.innerHTML = `${element.checkpoint_name}`;
            checkpoints.appendChild(row);
            row.addEventListener('click', () => {
                checkpoint_name = this.dataset.checkpoint;
                finetune_activate();
            });
        });
    }
}

function finetune_activate() {
    fetch("/tab-finetune-activate", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            "model": selected_model,
            "lora_run_id": selected_lora,
            "checkpoint": checkpoint_name
        })
    })
    .then(function (response) {
        console.log(response);
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
        save_finetune_settings();
    });
}
function save_finetune_settings() {
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
        // let partialLine = '';
        const response = await fetch(`/tab-finetune-log/${run_id}`);
        const reader = response.body.getReader();
        logstream_reader = reader;
        logstream_runid = run_id;
        let refresh_timeout = null;

        const processResult = ({ done, value }) => {
            if (done) {
                console.log('Streaming complete');
                return;
            }

            const chunk = decoder.decode(value);
            // const lines = (partialLine + chunk).split('\n');

            log_div.textContent += chunk;

            // for (let i = 0; i < lines.length - 1; i++) {
            //     const line = lines[i];
            //     log_div.textContent += line + "\n";
            //     handle_auto_scroll();
            // }

            // partialLine = lines[lines.length - 1];

            // if (!refresh_timeout) {
            //     refresh_timeout = setTimeout(() => {
            //         render_runs();
            //         refresh_timeout = null;
            //     }, 1000);
            // }

            return reader.read().then(processResult);
        };

        return reader.read().then(processResult);
    };

    streamTextFile()
        .catch(error => {
            // Handle any errors that occur during the fetch
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

export function init() {
    const process_button = document.querySelector('.tab-finetune-run-now');
    process_button.addEventListener('click', function () {
        fetch("/tab-finetune-run-now")
            .then(function (response) {
            })
    });
}

export function tab_switched_here() {
    finetune_data();
    render_time_dropdown();
}
