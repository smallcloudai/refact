import { general_error } from './error.js';
import { init as init_upload_files_modal, switch_away as upload_files_modal_switch_away } from './components/modals/modal-upload-files.js'
import {get_spinner} from "./utils/utils.js";


let logs_streamer_run_id = "";
let gfx_showing_run_id = "";

let finetune_state,
    reference_finetune_state,
    finetune_configs_and_runs,
    reference_finetune_configs_and_runs,
    running_models_and_loras;

let selected_lora;
let finetune_settings_defaults = [];

let finetune_filter_panel,
    finetune_filter_button,
    finetune_filter_settings,
    finetune_filter_status,
    finetune_filter_progress,
    finetune_filter_error;

let finetune_panel,
    // finetune_button,
    finetune_settings;

let select_model_panel;

let current_accepted,
    current_rejected;


function tab_finetune_get() {
    fetch("tab-finetune-get")
    .then(function (response) {
        return response.json();
    })
    .then(function (data) {
        // console.log('tab-finetune-get',data);
        finetune_state = data;
    })
   .catch(function (error) {
        console.log('tab-finetune-get',error);
        general_error(error);
    });
}


export function get_finetune_config_and_runs() {
    return fetch("/tab-finetune-config-and-runs")
        .then(function (response) {
            if (!response.ok) {
                return response.json().then(function(json) {
                    throw new Error(json.detail);
                });
            }
            return response.json();
        })
        .catch(function (error) {
            console.log('tab-finetune-config-and-runs',error);
            general_error(error);
        });
}

function get_running_models_and_loras() {
    return fetch("/running-models-and-loras")
       .then(function (response) {
            if (!response.ok) {
                return response.json().then(function(json) {
                    throw new Error(json.detail);
                });
            }
            return response.json();
        })
      .catch(function (error) {
            console.log('tab-finetune-running-models-and-loras',error);
            general_error(error);
        });
}


function tab_finetune_config_and_runs() {
    get_finetune_config_and_runs().then((data) => {
        get_running_models_and_loras().then((running_data) => {
            if (!data) {
                return;
            }
            finetune_configs_and_runs = data;
            running_models_and_loras = running_data;
            render_runs();
            render_model_select();
            render_finetune_settings(data);
            finetune_controls_state();
        });
    });
}

function rename_post(run_id, new_name) {
    return fetch("/tab-finetune-rename", {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            run_id_old: run_id,
            run_id_new: new_name
        })
    })
    .then(function (response) {
        if (!response.ok) {
            return response.json().then(function(json) {
                throw new Error(json.detail);
            });
        }
        return true;
    })
    .catch(function (error) {
        console.log('tab-finetune-rename-run', error);
        general_error(error);
        return false;
    });
}

function render_model_select(force = false) {
    const model_selector = document.querySelector('#finetune-model');
    if (model_selector && model_selector.options.length > 0 && !force) {
        return;
    }
    fetch("/tab-host-models-get")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            console.log('tab-host-models-get',data);
            model_selector.innerHTML = '';
            data.models.forEach(model => {
                if(model.has_finetune && model.backend != "vllm") {
                    const new_option = new Option(model.name, model.name);
                    if(finetune_configs_and_runs.config.model_name === model.name) {
                        new_option.selected = true;
                    }
                    model_selector.appendChild(new_option);
                }
            })
        })
       .catch(function(error) {
            console.log('tab-host-models-get',error);
            general_error(error);
        });
}

// function render_finetune_settings(data = {}) {
    // if (data.config.auto_delete_n_runs) {
    //     document.querySelector('.store-input').value = data.config.auto_delete_n_runs;
    // }
    // if (data.config.limit_training_time_minutes) {
    //     const radio_limit_time = document.querySelector(`input[name="limit_training_time_minutes"][value="${data.config.limit_training_time_minutes}"]`);
    //     if (radio_limit_time) {
    //         radio_limit_time.checked = true;
    //     }
    // }
    // if (data.config.run_at_night) {
    //     document.querySelector('#night_run').checked = true;
    // }
    // if (data.config.run_at_night_time) {
    //     const selectElement = document.querySelector('.night-time');
    //     const optionToSelect = selectElement.querySelector(`option[value="${data.config.run_at_night_time}"]`);
    //     if (optionToSelect) {
    //         optionToSelect.selected = true;
    //     }
    // }
// }

function run_checked(run_id) {
    if (gfx_showing_run_id != run_id) {
        gfx_showing_run_id = run_id;
        const timestamp = new Date().getTime();
        const gfx = document.querySelector('.fine-gfx');
        gfx.src = `/tab-finetune-progress-svg/${run_id}?t=${timestamp}`;
    }
    start_log_stream(run_id);
    render_checkpoints(find_checkpoints_by_run(run_id));

    const log_link = document.querySelector('.log-link');
    if (log_link && log_link.classList.contains('d-none')) {
        log_link.classList.remove('d-none');
    }
    if (log_link) {
        log_link.href = `/tab-finetune-log/${run_id}`;
    }
}

function render_runs() {
    const runs_table = document.querySelector('.run-table');
    if (runs_table.dataset.hash == CryptoJS.MD5(JSON.stringify([finetune_configs_and_runs.finetune_runs, running_models_and_loras]))) {
        return;
    }
    if(finetune_configs_and_runs.finetune_runs.length === 0) {
        runs_table.innerHTML = '<tr><td>No runs yet.</td><td></td><td></td><td></td><td></td><td></td><td></td></tr>';
        return;
    }
    let finetune_is_working = false;
    let running_loras = [];
    for (let [k, v] of Object.entries(running_models_and_loras)) {
        for (let i of v) {
            if (i.includes(":")) {
                running_loras.push(i.split(":")[0] + ":" + i.split(":")[1]);
            }
        }
    }

    if(finetune_configs_and_runs.finetune_runs.length > 0) {
        runs_table.innerHTML = '';
        runs_table.dataset.hash = CryptoJS.MD5(JSON.stringify([finetune_configs_and_runs.finetune_runs, running_models_and_loras]));
    }
    finetune_configs_and_runs.finetune_runs.forEach(run => {
        const run_table_row = document.createElement('tr');
        run_table_row.classList.add('run-table-row');
        run_table_row.style.whiteSpace = 'nowrap';
        const run_name = document.createElement("td");
        const run_status = document.createElement("td");
        const run_minutes = document.createElement("td");
        const run_steps = document.createElement("td");
        const run_download = document.createElement("td");
        const run_delete = document.createElement("td");

        let run_status_div = document.createElement('div');
        run_status_div.style = "display: flex; justify-content: center; flex-direction: column;";

        let status_colors = {
            'preparing': 'warning',
            'starting': 'secondary',
            'working': 'secondary',
            'completed': 'success',
            'finished': 'success',
            'failed': 'danger',
        };

        let run_status_color = status_colors[run.status] || 'secondary';
        run_table_row.dataset.run = run.run_id;

        const run_is_working = !(['interrupted', 'failed', 'finished'].includes(run.status));

        let status_pill_div = document.createElement('div');
        status_pill_div.classList.add('ft-status-pill-div');
        status_pill_div.style.marginBottom = "3px";
        let status_pill = document.createElement('div');
        status_pill.className = `badge-square solid ${run_status_color}`;

        if (run_is_working) {
            let status_div = document.createElement('div');
            status_div.className = 'finetune-spinner spinner-border spinner-border-sm';
            status_div.role = 'status';
            status_pill.appendChild(status_div);
            status_pill.appendChild(document.createTextNode(run.status));
            if (!selected_lora) {
                selected_lora = run.run_id;
            }
        } else {
            status_pill.appendChild(document.createTextNode(run.status));
        }
        status_pill_div.appendChild(status_pill);
        run_status_div.appendChild(status_pill_div);


        if (run['deprecated']) {
            let deprecated_pill_div = document.createElement('div');
            deprecated_pill_div.classList.add('ft-status-pill-div')
            let deprecated_pill = document.createElement('div');
            deprecated_pill.classList.add('badge-square', 'secondary');
            deprecated_pill.innerText = 'deprecated';
            deprecated_pill_div.appendChild(deprecated_pill);
            run_status_div.appendChild(deprecated_pill_div);
        }
        run_status.appendChild(run_status_div);

        if (run.worked_minutes) {
            run_minutes.innerHTML = run.worked_minutes;
        }
        run_steps.innerHTML = run.worked_steps;

        const item_disabled = run_is_working ? "disabled" : ""
        const rename_disabled = running_loras.includes(`${run.model_name}:${run.run_id}`) ? "disabled" : "";

        run_name.innerHTML = `
            <div id="run_name_${run.run_id}" class="run-table-name" data-run="${run.run_id}" ${item_disabled}>
                <div id="run_div${run.run_id}" style="display: flex; flex-direction: row">
                    <div>
                         ${run.run_id}
                    </div>
                    <div>
                        <button class="run-rename btn btn-sm btn-hover btn-link"
                        data-run="${run.run_id}"
                        style="padding: 0; font-size: 0.9rem;" ${false}
                        ${rename_disabled}
                        ><i class="bi bi-pencil-square"></i></button>
                        <div class="run-rename-popup" data-run="${run.run_id}"><pre>Cannot rename: currently in use</pre></div>
                    </div>
                </div>
                <div id="run_div_rename${run.run_id}" class="run-table-rename" data-run="${run.run_id}" hidden>
                    <input type="text" id="run_rename_input${run.run_id}" value="${run.run_id}">
                    <button id="confirm_btn${run.run_id}" class="btn btn-sm btn-link"><i class="bi bi-check-lg"></i></button>
                    <button id="cancel_btn${run.run_id}" class="btn btn-sm btn-link"><i class="bi bi-x-lg"></i></button>
                </div>
                <span>${run.model_name}</span>
            </div>
        `

        run_delete.innerHTML = `<button class="btn btn-hover btn-outline-danger btn-sm" ${item_disabled}><i class="bi bi-trash3-fill"></i></button>`;
        if (find_checkpoints_by_run(run.run_id).length > 0) {
            run_download.innerHTML = `
                <a href="/lora-download?run_id=${run.run_id}"
                   download class="btn btn-hover btn-primary btn-sm" ${item_disabled}>
                <i class="bi bi-download"></i>
                </a>`;
            if (!run_is_working) {
                run_download.addEventListener('click', (event) => {
                    event.stopPropagation();
                });
            }
        }
        run_table_row.appendChild(run_name);
        run_table_row.appendChild(run_status);
        run_table_row.appendChild(run_minutes);
        run_table_row.appendChild(run_steps);
        run_table_row.appendChild(run_download);
        run_table_row.appendChild(run_delete);

        if (!run_is_working) {
            run_delete.addEventListener('click', () => {
                const lora_for_delete = run_table_row.dataset.run;
                let delete_lora_modal = document.getElementById('delete-lora-modal');
                let delete_lora_modal_button = delete_lora_modal.querySelector('.delete-lora-modal-submit');
                delete_lora_modal_button.dataset.lora = lora_for_delete;
                let delete_lora_modal_instance = bootstrap.Modal.getOrCreateInstance(delete_lora_modal);
                delete_lora_modal_instance.show();
            });
        }

        runs_table.appendChild(run_table_row);
        if (selected_lora == run.run_id) {
            run_table_row.classList.add('table-success');
            run_checked(run.run_id);
        }
    });
    const runs_table_rows = runs_table.querySelectorAll('tr');
    runs_table_rows.forEach(function (row) {
        row.addEventListener('click', function (event) {
            remove_runs_table_sucess();
            row.classList.add('table-success');
            event.stopPropagation();
            const run_id = this.dataset.run;
            selected_lora = run_id;
            document.querySelectorAll('.run-table-rename').forEach((rename_div) => {
                const div_run_id = rename_div.dataset.run;
                if (!rename_div.hidden && div_run_id !== run_id) {
                    const text_div = document.getElementById(`run_div${div_run_id}`);
                    const rename_input = document.getElementById(`run_rename_input${div_run_id}`);
                    rename_input.value = div_run_id;
                    rename_div.hidden = true;
                    text_div.hidden = false;
                }
            });
            run_checked(run_id);
        });
    });

    document.querySelectorAll(".run-rename").forEach((run_rename) => {

        if (run_rename.disabled) {
            run_rename.addEventListener('mouseover', () => {
            let popup_div = document.querySelector(`.run-rename-popup[data-run='${run_rename.dataset.run}']`);
                popup_div.style.display = 'block';
            });
            run_rename.addEventListener('mouseout', () => {
                let popup_div = document.querySelector(`.run-rename-popup[data-run='${run_rename.dataset.run}']`);
                popup_div.style.display = 'none';
            });
        }

        run_rename.addEventListener('click', (event) => {
            event.stopPropagation();

            const ready_to_rename_runs = finetune_configs_and_runs.finetune_runs.filter(
                run => run.run_id === run_rename.dataset.run
                && (['interrupted', 'failed', 'finished'].includes(run.status)));
            if (ready_to_rename_runs.length === 0) {
                return;
            }

            document.querySelectorAll('.run-table-rename').forEach((rename_div) => {
                const div_run_id = rename_div.dataset.run;
                const text_div = document.getElementById(`run_div${div_run_id}`);
                const rename_input = document.getElementById(`run_rename_input${div_run_id}`);
                rename_input.value = div_run_id;
                rename_div.hidden = true;
                text_div.hidden = false;
            });

            let rename_div = document.getElementById(`run_div_rename${run_rename.dataset.run}`);
            let text_div = document.getElementById(`run_div${run_rename.dataset.run}`);
            rename_div.hidden = false;
            text_div.hidden = true;

            let spinner = get_spinner();
            spinner.style.scale = "0.5";
            spinner.style.position = "absolute";

            let rename_input = document.getElementById(`run_rename_input${run_rename.dataset.run}`);
            const confirm_btn = document.getElementById(`confirm_btn${run_rename.dataset.run}`);
            const cancel_btn = document.getElementById(`cancel_btn${run_rename.dataset.run}`);

            let new_confirm_btn = document.createElement("button");
            new_confirm_btn.id = confirm_btn.id;
            new_confirm_btn.classList = confirm_btn.classList;
            new_confirm_btn.innerHTML = confirm_btn.innerHTML;
            confirm_btn.replaceWith(new_confirm_btn)

            new_confirm_btn.addEventListener('click', (event) => {
                new_confirm_btn.replaceWith(spinner);
                cancel_btn.hidden = true;
                rename_post(run_rename.dataset.run, rename_input.value).then((is_ok) => {
                    if (!is_ok) {
                        rename_input.value = run_rename.dataset.run;
                        spinner.replaceWith(new_confirm_btn);
                        cancel_btn.hidden = false;
                    }
                })
                tab_finetune_config_and_runs();
            });

            cancel_btn.addEventListener('click', (event) => {
                rename_input.value = run_rename.dataset.run;
                rename_div.hidden = true;
                text_div.hidden = false;
            });
        });
    });

}

function remove_runs_table_sucess() {
    const runs_table = document.querySelector('.run-table');
    runs_table.querySelectorAll('tr').forEach(function (row) {
        row.classList.remove('table-success');
    });
}

function delete_run(run_id) {
    fetch(`/tab-finetune-remove/${run_id}`)
    .then(response => {
        if (!response.ok) {
            return response.json()
        }
        const gfx = document.querySelector('.fine-gfx');
        gfx.src = `/tab-finetune-progress-svg/none`;
        const log_container = document.querySelector('.tab-upload-finetune-logs');
        if (log_container) {
            log_container.innerHTML = '';
        }
    })
    .catch(error => {
        console.log('tab-finetune-remove',error);
        general_error(error);
    });
}

const find_checkpoints_by_run = (run_id) => {
    const finetune_run = finetune_configs_and_runs.finetune_runs.find((run) => run.run_id === run_id);
    if (finetune_run) {
      return finetune_run.checkpoints;
    } else {
      return null;
    }
};

function render_checkpoints(data = []) {
    const checkpoints = document.querySelector('.table-checkpoints');
    checkpoints.innerHTML = '';
    if (data.length > 0) {
        data.forEach(element => {
            const row = document.createElement('tr');
            row.classList.add('align-middle');
            const cell = document.createElement('td');
            cell.textContent = `${element.checkpoint_name}`;
            cell.dataset.checkpoint = element.checkpoint_name;
            const download_cell = document.createElement('td');

            download_cell.innerHTML = `
                <a href="/lora-download?run_id=${selected_lora}&checkpoint_id=${element.checkpoint_name}"
                   download class="btn btn-hover btn-primary btn-sm">
                <i class="bi bi-download"></i>
                </a>`;

            row.appendChild(download_cell);
            row.appendChild(cell);

            checkpoints.appendChild(row);
        });
    }
}

// function render_schedule_dialog() {
//     const selectElement = document.querySelector('.night-time');
//     for (let hour = 0; hour < 24; hour++) {
//         const option = document.createElement("option");
//         const formattedHour = hour.toString().padStart(2, "0");

//         option.value = formattedHour + ":00";
//         option.text = formattedHour + ":00";
//         selectElement.appendChild(option);
//     }
// }
// const finetune_inputs = document.querySelectorAll('.fine-tune-input');
// for (let i = 0; i < finetune_inputs.length; i++) {
//     finetune_inputs[i].addEventListener('change', function () {
//         save_finetune_schedule();
//     });
// }
// function save_finetune_schedule() {
//     const data = {
//         "limit_training_time_minutes": document.querySelector('input[name="limit_training_time_minutes"]:checked').value,
//         "run_at_night": document.querySelector('#night_run').checked,
//         "run_at_night_time": document.querySelector('.night-time').value,
//         "auto_delete_n_runs": document.querySelector('.store-input').value,
//     }
//     console.log('save_finetune_settings', data);
//     fetch("/tab-finetune-config-save", {
//         method: "POST",
//         headers: {
//             'Content-Type': 'application/json'
//         },
//         body: JSON.stringify(data)
//     })
//     .then(function (response) {
//         console.log(response);
//         tab_finetune_get();
//     })
//    .catch(function (error) {
//         console.log('tab-finetune-config-save',error);
//         general_error(error);
//     });
// }

function get_finetune_settings(defaults = false) {
    fetch("/tab-finetune-training-get")
    .then(function(response) {
        return response.json();
    })
    .catch(error => {
        console.log('tab-finetune-training-get', error);
        general_error(error);
    })
    .then(function(data) {
        console.log('tab-finetune-training-get', data);
        let settings_data = null;
        finetune_settings_defaults = data.defaults;
        if(Object.keys(data.user_config).length > 0 && !defaults) {
            settings_data = data.user_config;
        } else {
            settings_data = data.defaults;
        }
        let YMD = new Date();
        let padZero = (num) => (num < 10 ? `0${num}` : num);
        let YMD2 = `lora-${YMD.getFullYear()}${padZero(YMD.getMonth() + 1)}${padZero(YMD.getDate())}-${padZero(YMD.getHours())}${padZero(YMD.getMinutes())}${padZero(YMD.getSeconds())}`;
        let ftname_input = document.querySelector('#finetune-tab-settings-modal #finetune_name')
        ftname_input.value = YMD2;
        ftname_input.setSelectionRange(0, 4);
        setTimeout(() => {
            ftname_input.focus();
        }, 100);
        if(settings_data.trainable_embeddings) {
            document.querySelector('#finetune-tab-settings-modal #trainable_embeddings1').checked = true;
        } else {
            document.querySelector('#finetune-tab-settings-modal #trainable_embeddings0').checked = true;
        }
        document.querySelector('#finetune-tab-settings-modal #lr').value = settings_data.lr;
        document.querySelector('#finetune-tab-settings-modal #batch_size').value = settings_data.batch_size;
        document.querySelector('#finetune-tab-settings-modal #warmup_num_steps').value = settings_data.warmup_num_steps;
        document.querySelector('#finetune-tab-settings-modal #weight_decay').value = settings_data.weight_decay;
        document.querySelector('#finetune-tab-settings-modal #train_steps').value = settings_data.train_steps;
        document.querySelector('#finetune-tab-settings-modal #lr_decay_steps').value = settings_data.lr_decay_steps;
        document.querySelector('#finetune-tab-settings-modal #lora_r').value = settings_data.lora_r;
        document.querySelector('#finetune-tab-settings-modal #lora_alpha').value = settings_data.lora_alpha;
        document.querySelector('#finetune-tab-settings-modal #lora_dropout').value = settings_data.lora_dropout;
        const low_gpu_mem_mode = settings_data.low_gpu_mem_mode;
        if (low_gpu_mem_mode) {
            document.querySelector('#finetune-tab-settings-modal #low_gpu_mem_mode_finetune').checked = true;
        } else {
            document.querySelector('#finetune-tab-settings-modal #low_gpu_mem_mode_finetune').checked = false;
        }
        // const trainable_embeddings = settings_data.trainable_embeddings;
        // if(trainable_embeddings) {
        //     document.querySelector('#finetune-tab-settings-modal #trainable_embeddings').checked = true;
        // } else {
        //     document.querySelector('#finetune-tab-settings-modal #trainable_embeddings').checked = false;
        // }
        // const use_heuristics = settings_data.use_heuristics;
        // if(use_heuristics) {
        //     document.querySelector('#finetune-tab-settings-modal #use_heuristics').checked = true;
        // } else {
        //     document.querySelector('#finetune-tab-settings-modal #use_heuristics').checked = false;
        // }
        // check_heuristics();
    })
}

function change_finetune_model() {
    let finetune_settings = finetune_configs_and_runs.config;
    fetch("/tab-finetune-training-setup", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            model_name: document.querySelector('#finetune-model').value,
            limit_time_seconds: finetune_settings.limit_time_seconds,
            lr: finetune_settings.lr,
            batch_size: finetune_settings.batch_size,
            warmup_num_steps: finetune_settings.warmup_num_steps,
            weight_decay: finetune_settings.weight_decay,
            use_heuristics: finetune_settings.use_heuristics,
            train_steps: finetune_settings.train_steps,
            lr_decay_steps: finetune_settings.lr_decay_steps,
            lora_r: finetune_settings.lora_r,
            lora_alpha: finetune_settings.lora_alpha,
            lora_dropout: finetune_settings.lora_dropout,
            low_gpu_mem_mode: finetune_settings.low_gpu_mem_mode,
        })
    })
    .then(function(response) {
        if(!response.ok) {
            return response.json();
        }
        tab_finetune_config_and_runs();
        render_checkpoints();
        document.querySelector('.fine-gfx').src = `/tab-finetune-progress-svg/none`;
        document.querySelector('.tab-upload-finetune-logs').textContent = '';
    })
    .catch(error => {
        console.log('tab-finetune-training-setup', error);
        general_error(error);
    });
}

function save_finetune_settings() {
    // console.log('save_finetune_settings');
    let low_gpu = false;
    let trainable_embeddings = false;
    if (document.querySelector('#finetune-tab-settings-modal #low_gpu_mem_mode_finetune').checked) {
        low_gpu = true;
    }
    if (document.querySelector('#finetune-tab-settings-modal #trainable_embeddings1').checked) {
        trainable_embeddings = true;
    }
    // let use_heuristics = false;
    // if (document.querySelector('#finetune-tab-settings-modal #use_heuristics').checked) {
    //     use_heuristics = true;
    // }
    let launch_gpu0 = document.querySelector('#finetune-tab-settings-modal #launch_gpu0').checked;
    let launch_gpu1 = document.querySelector('#finetune-tab-settings-modal #launch_gpu1').checked;
    let launch_gpu2 = document.querySelector('#finetune-tab-settings-modal #launch_gpu2').checked;
    let launch_gpu3 = document.querySelector('#finetune-tab-settings-modal #launch_gpu3').checked;
    let launch_gpu4 = document.querySelector('#finetune-tab-settings-modal #launch_gpu4').checked;
    let launch_gpu5 = document.querySelector('#finetune-tab-settings-modal #launch_gpu5').checked;
    let launch_gpu6 = document.querySelector('#finetune-tab-settings-modal #launch_gpu6').checked;
    let launch_gpu7 = document.querySelector('#finetune-tab-settings-modal #launch_gpu7').checked;
    let gpus = [];
    if (launch_gpu0) gpus.push(0);
    if (launch_gpu1) gpus.push(1);
    if (launch_gpu2) gpus.push(2);
    if (launch_gpu3) gpus.push(3);
    if (launch_gpu4) gpus.push(4);
    if (launch_gpu5) gpus.push(5);
    if (launch_gpu6) gpus.push(6);
    if (launch_gpu7) gpus.push(7);

    fetch("/tab-finetune-training-setup", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            run_id: document.querySelector('#finetune-tab-settings-modal #finetune_name').value,
            model_name: document.querySelector('#finetune-model').value,
            // limit_time_seconds: document.querySelector('#finetune-tab-settings-modal #limit_time_seconds').value,
            trainable_embeddings: trainable_embeddings,
            low_gpu_mem_mode: low_gpu,
            lr: document.querySelector('#finetune-tab-settings-modal #lr').value,
            batch_size: document.querySelector('#finetune-tab-settings-modal #batch_size').value,
            warmup_num_steps: document.querySelector('#finetune-tab-settings-modal #warmup_num_steps').value,
            weight_decay: document.querySelector('#finetune-tab-settings-modal #weight_decay').value,
            // use_heuristics: use_heuristics,
            train_steps: document.querySelector('#finetune-tab-settings-modal #train_steps').value,
            lr_decay_steps: document.querySelector('#finetune-tab-settings-modal #lr_decay_steps').value,
            lora_r: document.querySelector('#finetune-tab-settings-modal #lora_r').value,
            lora_alpha: document.querySelector('#finetune-tab-settings-modal #lora_alpha').value,
            lora_dropout: document.querySelector('#finetune-tab-settings-modal #lora_dropout').value,
            gpus: gpus,
        })
    })
    .then(function(response) {
        if(!response.ok) {
            return response.json();
        }
        const finetune_settings_error = document.querySelector('.finetune-settings-error');
        finetune_settings_error.textContent = '';
        finetune_settings_error.classList.add('d-none');
        get_finetune_settings();
        let url_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('finetune-tab-settings-modal'));
        url_modal.hide();

    })
    .catch(error_data => {
        const finetune_settings_error = document.querySelector('.finetune-settings-error');
        let error_text = '';

        error_data.detail.forEach((error) => {
            const field_name = error.loc[1];
            const error_message = error.msg;
            const field_text = `${field_name}: ${error_message}`;
            error_text += field_text + '<br>';
        });

        finetune_settings_error.innerHTML = error_text;
        finetune_settings_error.classList.remove('d-none');
    });
}

// function check_heuristics() {
//     const finetune_use_heuristics = document.querySelector('#use_heuristics');
//     if(!finetune_use_heuristics.checked) {
//         document.querySelector('.finetune-settings-optional').classList.remove('finetune-settings-optional-disabled');
//         document.querySelectorAll('.finetune-settings-optional input').forEach(element => {
//             element.removeAttribute('tabindex');
//         });
//     } else {
//         document.querySelector('.finetune-settings-optional').classList.add('finetune-settings-optional-disabled');
//         document.querySelectorAll('.finetune-settings-optional input').forEach(element => {
//             element.setAttribute('tabindex', '-1');
//         });
//     }
// }

function revert_to_default(input_id) {
    const input = document.getElementById(input_id);
    input.value = finetune_settings_defaults[input_id];
}

function filtering_button_clicked() {
    if(!finetune_state) { return; }
    // filter not working - start
    if(!finetune_state.filter_working_now && !finetune_state.finetune_working_now) {
        reset_ftf_progress();
        if(!document.querySelector('.sources-run-button .spinner-border')) {
            finetune_filter_button.innerHTML = `<span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span></i>Starting`;
            finetune_filter_status.innerHTML = 'starting';
        }
        start_filtering();
    }
    // filter working - stop
    if(finetune_state.filter_working_now && !finetune_state.finetune_working_now) {
        // finetune_button.innerHTML = `Stopping...`;
        stop_filtering();
    }
}

function start_filtering() {
    fetch("/tab-finetune-run-now?filter_only=1")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        console.log('start_filtering');
    })
    .catch(error => {
        console.log('tab-finetune-run-now?filter_only=1',error);
        general_error(error);
    });
}

function stop_filtering() {
    fetch("/tab-finetune-stop-now")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        console.log('stop_filtering');
    })
   .catch(error => {
        console.log('tab-finetune-stop-now',error);
        general_error(error);
    });
}

function render_ftf_stats(data) {
    const ftf_wrapper = document.querySelector('.ftf-stats');
    if(Object.keys(data).length > 0 && data.accepted !== undefined && data.rejected !== undefined && data.worked_steps > 0) {
        current_accepted = data.accepted;
        current_rejected = data.rejected;
        ftf_wrapper.innerHTML = '';
        const content = `<h6>GPU Filtering stats</h6><div style="display:flex;"><div class="margin-right: 20px;">Accepted: ${data.accepted} <a target="_blank" href="/tab-finetune-filter-log?accepted_or_rejected=accepted">Full list</a></div><div>Rejected: ${data.rejected} <a target="_blank" href="/tab-finetune-filter-log?accepted_or_rejected=rejected">Full list</a></div></div>`;
        ftf_wrapper.innerHTML = content;
        const total_steps = data.total_steps;
        const working_steps = data.worked_steps;
        const percentage = (Number(working_steps + 1) / Number(total_steps)) * 100;
        render_ftf_progress(percentage);
    } else {
        reset_ftf_progress();
    }
}

function render_ftf_progress(filtering_progress) {
    const ftf_bar = document.querySelector('.ftf-bar');
    ftf_bar.style.width = filtering_progress + "%";
}

function reset_ftf_progress() {
    const fine_filter_status = document.querySelector('.ftf-status span');
    fine_filter_status.innerHTML = '';
    const fine_filter_stats = document.querySelector('.ftf-stats');
    fine_filter_stats.innerHTML = '';
    const eta_state = document.querySelector('.ftf-eta');
    eta_state.innerHTML = '';
    const progress_container = document.querySelector('.ftf-progress');
    progress_container.classList.add('d-none');
    const ftf_bar = document.querySelector('.ftf-bar');
    ftf_bar.style.width = "0%";
    const error = document.querySelector('.ftf-error');
    error.classList.add('d-none');
    error.innerHTML = 'Error:<span class="text-danger"></span>';
}

function render_ftune_stats(run) {
    const worked_minutes = Number(run.worked_minutes);
    const total_steps = run.total_steps;
    const working_steps = run.worked_steps;
    const eta_minutes = run.eta_minutes;
    const percentage = (Math.max(1, Number(working_steps)) / Number(total_steps)) * 100;
    render_ftune_progress(percentage, eta_minutes);
}

function render_ftune_progress(ftune_progress, ftune_eta) {
    const progress_container = document.querySelector('.ftune-progress');
    progress_container.classList.remove('d-none');
    const ftune_bar = document.querySelector('.ftune-bar');
    ftune_bar.style.width = ftune_progress + "%";
    if (ftune_eta !== undefined) {
        const eta_state = document.querySelector('.ftune-eta');
        eta_state.innerHTML = 'ETA: ' + ftune_eta + ' minute(s)';
    }
    const ftune_stats = document.querySelector('.start-finetune-stats');
    ftune_stats.classList.remove('d-none');
}

function reset_ftune_progress() {
    const ftune_progress = document.querySelector('.ftune-progress');
    ftune_progress.classList.add('d-none');
    const ftune_bar = document.querySelector('.ftune-bar');
    ftune_bar.style.width = "0%";
    const eta_state = document.querySelector('.ftune-eta');
    eta_state.innerHTML = '';
    const ftune_stats = document.querySelector('.start-finetune-stats');
    ftune_stats.classList.add('d-none');
}

function get_filters_settings(defaults = false) {
    fetch("/tab-finetune-smart-filter-get")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        console.log('tab-finetune-smart-filter-get',data);
        let settings_data = null;
        if(Object.keys(data.user_config).length > 0 && !defaults) {
            settings_data = data.user_config;
        } else {
            settings_data = data.defaults;
        }
        document.querySelector('#upload-tab-source-settings-modal #filter_loss_threshold').value = settings_data.filter_loss_threshold;
    })
   .catch(error => {
        console.log('tab-finetune-smart-filter-get',error);
        general_error(error);
    });
}

function save_filters_settings() {
    fetch("/tab-finetune-smart-filter-setup", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            filter_loss_threshold: Number(document.querySelector('#upload-tab-source-settings-modal #filter_loss_threshold').value),
        })
    })
    .then(function(response) {
        if(response.ok) {
            get_filters_settings();
        }
    })
   .catch(error => {
        console.log('tab-finetune-smart-filter-setup',error);
        general_error(error);
    });
}

function handle_auto_scroll() {
    if (log_container.scrollHeight - log_container.scrollTop === log_container.clientHeight) {
        log_container.scrollTop = log_container.scrollHeight;
    }
}

function finetune_controls_state()
{
    if(!finetune_state) { return }
    if(!reference_finetune_state) { reference_finetune_state = finetune_state; }
    if(!reference_finetune_configs_and_runs) { reference_finetune_configs_and_runs = finetune_configs_and_runs; }
    if(finetune_state === reference_finetune_state && finetune_configs_and_runs === reference_finetune_configs_and_runs) { return }
    const progress_container = document.querySelector('.ftf-progress');
    const eta_state = document.querySelector('.ftf-eta');
    const ftf_bar = document.querySelector('.ftf-bar');

    // "prog_name": ["prog_linguist", "prog_filter", "prog_ftune"],
    // "prog_status": ["starting", "working", "finished", "failed", "interrupted", "idle"]
    let prog_status = finetune_state.prog_status;
    let prog_name = finetune_state.prog_name;
    const working_or_starting = prog_status === "starting" || prog_status === "working";
    const show_stop = prog_status === "starting" || prog_status === "working";
    const can_start = prog_status === "finished" || prog_status === "failed" || prog_status === "interrupted" || prog_status === "idle";
    finetune_filter_settings.disabled = working_or_starting;
    const linguist_working_or_starting = prog_name === "prog_linguist" && show_stop;

    if (linguist_working_or_starting) {
        finetune_panel.classList.add('pane-disabled');
        finetune_filter_panel.classList.add('pane-disabled');
        select_model_panel.classList.add('pane-disabled');
    } else {
        finetune_panel.classList.remove('pane-disabled');
        finetune_filter_panel.classList.remove('pane-disabled');
        select_model_panel.classList.remove('pane-disabled');
    }

    if (prog_name === "prog_filter" && prog_status === "working") {
        progress_container.classList.remove('d-none')
        eta_state.innerHTML = 'ETA: ' + finetune_state.finetune_filter_stats.eta_minutes + ' minute(s)';
    } else {
        progress_container.classList.remove('d-none');
        eta_state.innerHTML = '';
    }

    let can_stop_filter = prog_name === "prog_filter" && show_stop;
    if (can_stop_filter) {
        finetune_filter_button.innerHTML = `<span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span></i> Stop Filtering`;
        finetune_filter_button.setAttribute("need_to_stop", true);
    } else {
        finetune_filter_button.innerHTML = `<i class="bi bi-funnel-fill"></i> Run Filter`;
        finetune_filter_button.setAttribute("need_to_stop", false);
    }
    finetune_filter_button.disabled = !(can_start || can_stop_filter);

    // let can_stop_ftune = prog_name === "prog_ftune" && show_stop;
    // if (can_stop_ftune) {
    //     finetune_button.innerHTML = '<div class="upload-spinner spinner-border spinner-border-sm" role="status"></div>' + 'Stop';
    //     finetune_button.setAttribute("need_to_stop", true);
    // } else {
    //     finetune_button.innerHTML = `<i class="bi bi-gpu-card"></i> Run Finetune`;
    //     finetune_button.setAttribute("need_to_stop", false);
    // }
    // finetune_button.disabled = !(can_start || can_stop_ftune);

    render_ftf_stats(finetune_state.finetune_filter_stats);

    if(finetune_state.finetune_filter_stats.filtering_status) {
        document.querySelector('.ftf-status').classList.remove('d-none');
        document.querySelector('.ftf-status span').innerHTML = finetune_state.finetune_filter_stats.filtering_status;
    } else {
        document.querySelector('.ftf-status').classList.add('d-none');
    }

    let error_span = document.querySelector('.ftf-error span');
    let ftf_error = document.querySelector('.ftf-error');
    if (finetune_state.finetune_filter_stats.filtering_status == "failed") {
        ftf_error.classList.remove('d-none');
        if(finetune_state.finetune_filter_stats.error && finetune_state.finetune_filter_stats.error !== '') {
            error_span.innerHTML = finetune_state.finetune_filter_stats.error;
        }
    } else {
        ftf_error.classList.add('d-none');
        error_span.innerHTML = '';
    }
    if(finetune_state.prog_name === 'prog_filter' && finetune_state.prog_status === 'failed') {
        document.querySelector('.ftf-status span').innerHTML = finetune_state.prog_status;
    }

    // example:
    // "finetune_filter_stats": {
    //     "filtering_status": "failed",
    //     "total_steps": 116,
    //     "worked_steps": 115,
    //     "worked_minutes": 0,
    //     "eta_minutes": 0,
    //     "accepted": 111,
    //     "rejected": 5,
    //     "avg_loss": 1.1812065972222223,
    //     "error": "_update_and_dump_status() missing 1 required positional argument: 'new_status'"
    // },

    const runs = finetune_configs_and_runs.finetune_runs.filter(run => run.status === "working");
    if (runs.length > 0) {
        render_ftune_stats(runs[runs.length - 1]);
    } else {
        reset_ftune_progress();
    }
}

let logs_streamer_to_stop = undefined;

function start_log_stream(run_id) {
    if (logs_streamer_run_id == run_id || run_id === "") {
        console.log(`already streaming "${run_id}"`);
        return;
    }
    const streamUrl = `/tab-finetune-log/${run_id}`;
    const streamDiv = document.querySelector('.tab-upload-finetune-logs');
    streamDiv.textContent = "";
    const gfx = document.querySelector('.fine-gfx');
    let gfx_updated_ts = new Date().getTime();
    const fetchData = async () => {
        const response = await fetch(streamUrl);
        if (!response.ok) {
            throw new Error(`start_log_stream (1): ${response.status}`);
        }
        const reader = response.body.getReader();
        if (logs_streamer_to_stop !== undefined) {
            // it's stuck on read() most likely, we need to get it out of that call
            await logs_streamer_to_stop.cancel();
        }
        logs_streamer_to_stop = reader;
        logs_streamer_run_id = run_id;
        try {
            while (1) {
                const { done, value } = await reader.read();
                if (done) {
                    logs_streamer_run_id = "";
                    reader.cancel();
                    return;
                }
                let now = new Date().getTime();
                if (gfx_updated_ts + 1000 < now && gfx_showing_run_id == run_id) {
                    gfx.src = `/tab-finetune-progress-svg/${run_id}?t=${now}`;
                    gfx_updated_ts = now;
                }
                const data_decoded = new TextDecoder('utf-8').decode(value);
                const isAtBottom = streamDiv.scrollTop >= (streamDiv.scrollHeight - streamDiv.offsetHeight);
                streamDiv.textContent += data_decoded;
                if (isAtBottom) {
                    streamDiv.scrollTop = streamDiv.scrollHeight;
                }
            }
        } catch (error) {
            console.error(`start_log_stream (2): ${error}`);
        } finally {
            await reader.cancel();
            logs_streamer_to_stop = undefined;
            logs_streamer_run_id = "";
        }
    };
    fetchData();
}


export async function init() {
    let req = await fetch('/tab-finetune.html');
    document.querySelector('#finetune').innerHTML = await req.text();

    finetune_filter_panel = document.querySelector('.start-funetune-step1');
    finetune_filter_panel.classList.add('pane-disabled');
    finetune_filter_progress = document.querySelector('.start-filter-stats .progress-bar');
    finetune_filter_settings = document.querySelector('.sources-settings');
    finetune_filter_status = document.querySelector('.ftf-status span');
    finetune_filter_error = document.querySelector('.ftf-error');
    finetune_filter_button = document.querySelector('.sources-run-button');
    finetune_filter_button.addEventListener('click', filtering_button_clicked);

    finetune_panel = document.querySelector('.start-funetune-step2');
    finetune_panel.classList.add('pane-disabled');
    // finetune_button = document.querySelector('.tab-finetune-run-now');
    finetune_settings = document.querySelector('.tab-finetune-fine-settings');

    select_model_panel = document.querySelector('.start-funetune-select-model');

    const log_container = document.querySelector('.log-container');
    function handle_auto_scroll() {
        if (log_container.scrollHeight - log_container.scrollTop === log_container.clientHeight) {
            log_container.scrollTop = log_container.scrollHeight;
        }
    }

    log_container.addEventListener('scroll', handle_auto_scroll);

    // const start_finetune_button = document.querySelector('.tab-finetune-run-now');
    // start_finetune_button.addEventListener('click', function () {
    //     let url = "/tab-finetune-run-now";
    //     start_finetune_button.disabled = true;
    //     if (start_finetune_button.getAttribute("need_to_stop") === 'true') {
    //         url = "/tab-finetune-stop-now";
    //     } else {
    //         start_finetune_button.innerHTML = `<div class="upload-spinner spinner-border spinner-border-sm" role="status"></div>Starting...`;
    //     }
    //     fetch(url)
    //         .then(function (response) {
    //             tab_finetune_get();
    //         })
    // });

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
    let delete_lora_modal_button = document.querySelector('.delete-lora-modal-submit');
    delete_lora_modal_button.addEventListener('click', () => {
        const lora_for_delete = delete_lora_modal_button.dataset.lora;
        delete_run(lora_for_delete);
        let delete_lora_modal = document.getElementById('delete-lora-modal');
        let delete_lora_modal_instance = bootstrap.Modal.getOrCreateInstance(delete_lora_modal);
        delete_lora_modal_instance.hide();
    });

    // check_heuristics();
    // const finetune_use_heuristics = document.querySelector('#use_heuristics');
    // finetune_use_heuristics.addEventListener('change', function(event) {
    //     check_heuristics();
    // });

    const finetune_default_buttons = document.querySelectorAll('.form-clear-default');
    finetune_default_buttons.forEach(element => {
        element.addEventListener('click', function(event) {
            revert_to_default(event.target.parentNode.previousElementSibling.id);
        });
    });

    const settings_modal = document.getElementById('upload-tab-source-settings-modal');
    settings_modal.addEventListener('show.bs.modal', function () {
        get_filters_settings();
    });

    const settings_modal_submit = document.querySelector('.tab-upload-source-settings-submit');
    settings_modal_submit.addEventListener('click', function() {
        save_filters_settings();
        const settings_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-tab-source-settings-modal'));
        settings_modal.hide();
    });

    const settings_modal_defaults = document.querySelector('.tab-upload-source-settings-default');
    settings_modal_defaults.addEventListener('click', function() {
        get_filters_settings(true);
    });

    const model_select_dropdown = document.querySelector('#finetune-model');
    model_select_dropdown.addEventListener('change', function() {
        change_finetune_model();
    });

}

export function tab_switched_here() {
    tab_finetune_get();
    tab_finetune_config_and_runs();
    // render_schedule_dialog();
    init_upload_files_modal(
        document.querySelector('#lora-upload-files-modal'),
        document.querySelector('#finetune-upload-lora-open-modal'),
        'Upload Lora',
        'link',
        '/lora-upload-url', '/lora-upload',
        "Loading lora. This may take a few more minutes..."
    );
}

export function tab_switched_away() {
    if (logs_streamer_to_stop !== undefined) {
        logs_streamer_to_stop.cancel();
        logs_streamer_to_stop = undefined;
    }
    upload_files_modal_switch_away(document.querySelector('#lora-upload-files-modal'));
}

export function tab_update_each_couple_of_seconds() {
    tab_finetune_get();
    tab_finetune_config_and_runs();
}
