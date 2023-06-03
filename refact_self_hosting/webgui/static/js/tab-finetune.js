let stream_running = false;
function finetune_data() {
    fetch("/tab-finetune-config-and-runs")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        console.log(data);
        render_finetune_settings(data);
        render_runs(data);
    });
}

function render_finetune_settings(data = {}) {
    if(data.config.auto_delete_n_runs) {
        document.querySelector('.store-input').value = data.config.auto_delete_n_runs;
    }
    if(data.config.limit_training_time_minutes) {
        const radio_limit_time = document.querySelector(`input[name="limit_training_time_minutes"][value="${data.config.limit_training_time_minutes}"]`);
        if(radio_limit_time) {
            radio_limit_time.checked = true;
        }
    }
    if(data.config.run_at_night) {
        document.querySelector('#night_run').checked = true;
    }
    if(data.config.run_at_night_time) {
        const selectElement = document.querySelector('.night-time');
        const optionToSelect = selectElement.querySelector(`option[value="${data.config.run_at_night_time}"]`);
        if(optionToSelect) {
            optionToSelect.selected = true;
        }
    }
}

function render_runs(data = {}) {
    document.querySelector('.run-table').innerHTML = '';
    let i = 0;
    data.finetune_runs.forEach(element => {
        const row = document.createElement('tr');
        const run_name = document.createElement("td");
        const run_minutes = document.createElement("td"); 
        const run_steps = document.createElement("td");

        run_name.innerHTML = element.run_id;
        row.dataset.run = element.run_id;
        run_minutes.innerHTML = element.worked_minutes;
        run_steps.innerHTML = element.worked_steps;
        row.appendChild(run_name);
        row.appendChild(run_minutes);
        row.appendChild(run_steps);
        document.querySelector('.run-table').appendChild(row);
        const rows = document.querySelectorAll('.run-table tr');
        rows.forEach(function(row) {
            row.addEventListener('click', function() {
                rows.forEach(function(row) {
                    row.classList.remove('table-primary');
                });
                this.classList.add('table-primary');
                const run_id = this.dataset.run;
                document.querySelector('.fine-gfx').src = `/tab-finetune-progress-svg/${run_id}`;
                if(!stream_running) {
                    render_log_stream(run_id);
                }
            });
        });
        i++;
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
    finetune_inputs[i].addEventListener('change', function() {
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
    .then(function(response) {
        console.log(response);
        finetune_data();
    });
}

function render_log_stream(id) {
    const run_id = id;
    const log_div = document.querySelector('.tab-upload-finetune-logs');

    fetch(`/tab-finetune-log/${run_id}`)
    .then(response => {
        if (!response.ok) {
        throw new Error('Network response was not ok');
        }

        const reader = response.body.getReader();
        const decoder = new TextDecoder('utf-8');
        let partialLine = '';

        const processChunk = ({ done, value }) => {
            if (!stream_running) {
                console.log('Function stopped');
                return;
            }
            if (done) {
                console.log('Streaming complete');
                return;
            }
            stream_running = true;

            const chunk = decoder.decode(value, { stream: true });
            const lines = (partialLine + chunk).split('\n');

            for (let i = 0; i < lines.length - 1; i++) {
                const line = lines[i];
                const content_div = document.createElement('div');
                content_div.textContent = line;
                log_div.appendChild(content_div);
            }

            partialLine = lines[lines.length - 1];

            finetune_data();

            reader.read().then(processChunk);
        };

        reader.read().then(processChunk);
    })
    .catch(error => {
        console.error('Error:', error);
    });
}

export function init() {
    finetune_data();
    render_time_dropdown();
}