function get_tab_files() {
    fetch("/tab-files-get")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            console.log('tab-files-get',data);
            switch(data.filtering_stage) {
                case 0:
                    filter_state_zero();
                    break;
                case 1:
                    filter_state_one();
                    break;
                case 2:
                    filter_state_two();
                    render_filter_progress(data.filtering_progress);
                    break;

            }
            render_tab_files(data);
            render_filetypes(data.mime_types);
            render_filter_setup_defaults(data.filter_setup_defaults);
        });
}

let sources_settings_modal = false;
let sources_filtypes_changed = false;
const progress_bar = document.querySelector('.sources-run-progress .progress-bar');
const sidebar_progress = document.querySelector('.sources-sidebar-progress .progress-bar');
const sidebar_step1 = document.querySelector('.sources-list li:first-of-type');
const sidebar_step2 = document.querySelector('.sources-list li:last-of-type');
const sources_pane = document.querySelector('.sources-pane');
const filetypes_pane = document.querySelector('.filetypes-pane');
const sources_run_pane = document.querySelector('.run-pane');
const sources_run_button = document.querySelector('.sources-run-button');
const sources_run_progress = document.querySelector('.sources-run-progress');
const summary_pane = document.querySelector('.summary-pane');
const sources_settings = document.querySelector('.sources-settings');

function render_filter_progress(progress_value) {
    progress_bar.style.width = progress_value + "%";
}
// function render_filter_button(value) {
//     if(value === "completed") {
//         sources_run_button.innerHTML = `<i class="bi bi-gpu-card"></i>${value}`;
//     } else {
//         sources_run_button.innerHTML = `<i class="bi bi-gpu-card"></i>Run filter`;
//     }
// }
function filter_state_zero() {
    progress_bar.style.width = "0%";
    sidebar_progress.style.width = "0%";
    sources_run_progress.classList.add('d-none');
    sources_run_button.innerHTML = `<i class="bi bi-gpu-card"></i>Run filter`;
    sidebar_step1.classList.add('sources-list-active');
    sidebar_step2.classList.remove('sources-list-active');
    sources_pane.classList.remove('pane-disabled');
    filetypes_pane.classList.add('pane-disabled');
    sources_run_pane.classList.add('pane-disabled');
    sources_settings.classList.remove('pane-disabled');
}

function filter_state_one() {
    progress_bar.style.width = "0%";
    sidebar_progress.style.width = "50%";
    sources_run_progress.classList.add('d-none');
    sources_run_button.innerHTML = `<i class="bi bi-gpu-card"></i>Run filter`;
    sidebar_step1.classList.add('sources-list-active');
    sidebar_step2.classList.add('sources-list-active');
    sources_pane.classList.remove('pane-disabled');
    filetypes_pane.classList.remove('pane-disabled');
    sources_run_pane.classList.remove('pane-disabled');
    sources_settings.classList.remove('pane-disabled');
}

function filter_state_two() {
    progress_bar.style.width = "0%";
    sidebar_progress.style.width = "100%";
    sources_run_progress.classList.add('d-none');
    sources_run_button.innerHTML = `<span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span>Stop`;
    sidebar_step1.classList.add('sources-list-active');
    sidebar_step2.classList.add('sources-list-active');
    sources_pane.classList.add('pane-disabled');
    filetypes_pane.classList.add('pane-disabled');
    sources_run_pane.classList.remove('pane-disabled');
    sources_settings.classList.add('pane-disabled');
}

function render_tab_files(data) {
    const files = document.getElementById("upload-tab-table-body-files");
    files.innerHTML = "";
    let i = 0;
    for(let item in data.uploaded_files) {
        const row = document.createElement('tr');
        row.setAttribute('data-file', item);
        row.style.whiteSpace = 'nowrap';
        const name = document.createElement("td");
        const is_git = document.createElement("td");
        const status = document.createElement("td");
        const set = document.createElement("td");
        const delete_file = document.createElement("td");
        name.innerHTML = item;

        const which_set = data.uploaded_files[item].which_set;
        if(which_set === "train") {
            // TODO XXX : lora-input?
            set.innerHTML = `<div class="btn-group" role="group" aria-label="basic radio toggle button group"><input type="radio" class="file-radio btn-check" name="file-which[${i}]" id="file-radio-auto${i}" value="train" autocomplete="off" checked><label for="file-radio-auto${i}" class="btn btn-outline-primary">Auto</label><input type="radio" class="lora-input btn-check" name="file-which[${i}]" value="test" id="file-radio-test${i}" autocomplete="off"><label for="file-radio-test${i}" class="btn btn-outline-primary">Test set</label></div>`
        }
        if(which_set === "test") {
            set.innerHTML = `<div class="btn-group" role="group" aria-label="basic radio toggle button group"><input type="radio" class="file-radio btn-check" name="file-which[${i}]" id="file-radio-auto${i}" value="train" autocomplete="off"><label for="file-radio-auto${i}" class="btn btn-outline-primary">Auto</label><input type="radio" class="lora-input btn-check" name="file-which[${i}]" value="test" id="file-radio-test${i}" autocomplete="off" checked><label for="file-radio-test${i}" class="btn btn-outline-primary">Test set</label></div>`
        }
        delete_file.innerHTML = `<button type="button" data-file="${item}" class="btn btn-danger file-remove"><i class="bi bi-trash3-fill"></i></button>`;
        row.appendChild(name);
        row.appendChild(is_git);
        row.appendChild(status);
        row.appendChild(set);
        row.appendChild(delete_file);
        files.appendChild(row);
        i++;
    }

    change_events();
    delete_events();

    let any_working = false;
    for (const [item,item_object] of Object.entries(data.uploaded_files)) {
        const rows = files.querySelectorAll('tr');
        for (const row of rows) {
            const row_file_name = row.getAttribute('data-file');
            if (row_file_name === item) {
                const is_git_cell = row.querySelector('td:nth-child(2)');

                if (item_object.is_git) {
                    is_git_cell.innerHTML = `<span class="badge rounded-pill text-bg-warning">git</span>`;
                } else {
                    is_git_cell.innerHTML = `<span class="badge rounded-pill text-bg-info">file</span>`;
                }

                const target_cell = row.querySelector('td:nth-child(3)');
                let current_status = item_object.status;
                if (!current_status) {
                    current_status = "";
                }
                const status_color = file_status_color(current_status);
                let info_data = `<div><b>Status:</b> ${item_object.status}</div>`;
                if(item_object.files) {
                    info_data += `<div><b>Files:</b> ${item_object.files}</div>`;
                }
                if(item_object.generated) {
                    info_data += `<div><b>Generated:</b> ${item_object.generated}</div>`;
                }
                if(item_object.good) {
                    info_data += `<div><b>Good:</b> ${item_object.good}</div>`;
                }
                if(item_object.large) {
                    info_data += `<div><b>Too Large:</b> ${item_object.large}</div>`;
                }
                if(item_object.vendored) {
                    info_data += `<div><b>Vendored:</b> ${item_object.vendored}</div>`;
                }
                if(current_status === 'completed') {
                    // target_cell.innerHTML = `<span>Files: ${item_object.files} / Good: ${item_object.good}</span><i class="source-info bi bi-info-square-fill text-success"></i><div class="source-popup">${info_data}</div>`;
                    target_cell.innerHTML = `<span>${item_object.files} files</span><i class="source-info bi bi-info-square-fill text-success"></i><div class="source-popup">${info_data}</div>`;
                    row.querySelector('.source-info').addEventListener('mouseover', function(event) {
                        event.target.nextElementSibling.style.display = 'block';
                        // null on reading style
                    });
                    row.querySelector('.source-info').addEventListener('mouseout', function(event) {
                        // null on reading s
                        event.target.nextElementSibling.style.display = 'none';
                    });
                } else {
                    target_cell.innerHTML = `<span class="file-status badge rounded-pill ${status_color}">${current_status}</span>`;
                }
                if (current_status == "working" || current_status == "starting") {
                    any_working = true;
                }
                break;
            }
        }
    }

    const process_button = document.querySelector('.tab-files-process-now');
    if (any_working) {
        let process_button_text = "Stop";
        process_button.innerHTML = '<div class="upload-spinner spinner-border spinner-border-sm" role="status"></div>' + process_button_text;
    } else {
        if (process_button.dataset.loading) {
            process_button.dataset.loading = false;
            process_button.disabled = false;
        }
        process_button.innerHTML = "Scan sources";
    }
}

function get_ssh_keys() {
    fetch("/tab-settings-get-all-ssh-keys")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            console.log('get-all-ssh-keys',data);
        });
}

function delete_events() {
    document.querySelectorAll('#upload-tab-table-body-files .file-remove').forEach(function(element) {
        removeEventListener('click',element);
        element.addEventListener('click', function() {
            const file_name = this.getAttribute('data-file');
            let delete_modal = document.getElementById('delete-modal');
            let delete_modal_button = delete_modal.querySelector('.delete-modal-submit');
            delete_modal_button.dataset.file = file_name;
            let delete_modal_instance = bootstrap.Modal.getOrCreateInstance(delete_modal);
            delete_modal_instance.show();
        });
    });
}

function render_filetypes(data) {
    if(sources_filtypes_changed) {
        const file_types = document.querySelectorAll('.upload-tab-table-type-body tr input');
        let updated_data = [];
        if(file_types.length > 0) {
            const unchecked_inputs = Array.from(file_types).filter(input => !input.checked);
            unchecked_inputs.forEach(input => {
                updated_data.push({
                    'file_type': input.dataset.name,
                    'count': Number(input.value),
                    'suitable_to_train': false
                });
            });
            const checked_inputs = Array.from(file_types).filter(input => input.checked);
            checked_inputs.forEach(input => {
                updated_data.push({
                    'file_type': input.dataset.name,
                    'count': Number(input.value),
                    'suitable_to_train': true
                });
            });
        }
        render_stats(updated_data);
        return;
    }
    if(data && data.length > 0) {
        const table_body = document.querySelector('.upload-tab-table-type-body');
        table_body.innerHTML = '';
        let i = 0;
        data.forEach((item) => {
            const row = document.createElement('tr');
            let checkbox_checked = `checked`;
            const file_name = `<label for="file-list${i}">${item.file_type}</label>`;
            if(item.suitable_to_train) {
                row.classList.add('enabled-file');
            }
            if(!item.suitable_to_train) {
                row.classList.add('opacity-50');
                row.classList.add('disbled');
                checkbox_checked = `disabled`;
            }
            let file_checkbox = `<input id="file-list${i}" data-name="${item.file_type}" class="form-check-input" type="checkbox" value="${item.count}" ${checkbox_checked}>`;
            row.innerHTML = `<td>${file_checkbox}</td><td>${file_name}</td><td>${item.count}</td>`;
            table_body.appendChild(row);
            i++;
        });
        render_stats(data);
        watch_filetypes();
    }
}

function watch_filetypes() {
    const file_types = document.querySelectorAll('.upload-tab-table-type-body tr.enabled-file input:checked');
    if(file_types.length > 0) {
        file_types.forEach(function(element) {
            element.removeEventListener('change', function() {
                sources_filtypes_changed = true;
            });
            element.addEventListener('change', function() {
                sources_filtypes_changed = true;
            });
        });
    }
}

function render_stats(data) {
    let included_count = 0;
    let excluded_count = 0;
    data.forEach((item) => {
        if(item.suitable_to_train) {
            included_count += item.count;
        }
        if(!item.suitable_to_train) {
            excluded_count += item.count;
        }
    });
    const stat_included = document.querySelector('.sources-stats-inc');
    const stat_excluded = document.querySelector('.sources-stats-exc');
    stat_included.innerHTML = included_count;
    stat_excluded.innerHTML = excluded_count;
}

function upload_url() {
    const fileInput = document.querySelector('#tab-upload-url-input');
    if (!fileInput || fileInput.value === '') {
        return;
    }

    const url_regex = /^(ftp|http|https):\/\/[^ "]+$/;
    const is_url = url_regex.test(fileInput.value);
    if (!is_url) {
        handle_invalid_url();
        return;
    }

    const formData = {
        'url': fileInput.value
    };

    fetch('/tab-files-upload-url', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(formData)
    })
        .then(response => {
            if (!response.ok) {
                return response.json()
                    .then(error => {
                        throw new Error(error.message);
                    });
            }
            return response.json();
        })
        .then(data => {
            get_tab_files();
            fileInput.value = '';
            document.querySelector('#status-url').innerHTML = '';
            let url_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-tab-url-modal'));
            url_modal.hide();
        })
        .catch(error => {
            document.querySelector('#status-url').innerHTML = error.message;
        });
}

function handle_invalid_url() {
    const error = new Error('Invalid URL');
    document.querySelector('#status-url').innerHTML = error.message;
}


function upload_repo() {
    const gitUrl = document.querySelector('#tab-upload-git-input');
    const gitBranch = document.querySelector('#tab-upload-git-brach-input');
    if (!gitUrl || gitUrl.value == '') {
        return;
    }
    const formData = {
        'url': gitUrl.value
    };
    if (gitBranch.value && gitBranch.value !== '') {
        formData['branch'] = gitBranch.value;
    }

    fetch('/tab-files-repo-upload', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(formData)
    })
    .then(response => {
        if (!response.ok) {
            return response.json()
                .then(error => {
                    throw new Error(error.message);
                });
        }
        return response.text();
    })
    .then(data => {
        get_tab_files();
        gitUrl.value = '';
        gitBranch.value = '';
        let git_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-tab-git-modal'));
        git_modal.hide();
    })
    .catch(error => {
        document.querySelector('#status-git').innerHTML = error.message;
    });
}

function upload_file() {
    const fileInput = document.querySelector('#tab-upload-file-input');
    if(fileInput.files.length === 0) {
        return;
    }
    var formdata = new FormData();
    formdata.append("file", fileInput.files[0]);
    document.querySelector('.progress').classList.toggle('d-none');
    document.querySelector('.tab-upload-file-submit').classList.toggle('d-none');
    var ajax = new XMLHttpRequest();
    ajax.upload.addEventListener("progress", progressHandler, false);
    ajax.addEventListener("load", completeHandler, false);
    ajax.addEventListener("error", errorHandler, false);
    ajax.addEventListener("abort", abortHandler, false);
    ajax.open("POST", "/tab-files-upload");
    ajax.send(formdata);
}

function progressHandler(event) {
    document.querySelector('#loaded_n_total').innerHTML = "Uploaded " + event.loaded + " bytes of " + event.total;
    var percent = (event.loaded / event.total) * 100;
    document.querySelector('.progress-bar').setAttribute('aria-valuenow', Math.round(percent));
    document.querySelector('.progress-bar').style.width = Math.round(percent) + "%";
    document.querySelector('#status').innerHTML = Math.round(percent) + "% uploaded... please wait";
  }

  function completeHandler(event) {
    document.querySelector('#status').innerHTML = event.target.responseText;
    if(event.target.status === 200) {
        setTimeout(() => {
            get_tab_files();
            document.querySelector('#tab-upload-file-input').value = '';
            let file_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-tab-files-modal'));
            file_modal.hide();
        }, 500);
    } else {
        let error_msg = JSON.parse(event.target.responseText);
        const file_modal = document.getElementById('upload-tab-files-modal');
        file_modal.querySelector('.progress-bar').setAttribute('aria-valuenow', 0);
        file_modal.querySelector('.progress').classList.add('d-none');
        file_modal.querySelector('.tab-upload-file-submit').classList.remove('d-none');
        file_modal.querySelector('#loaded_n_total').innerHTML = "";
        file_modal.querySelector('#status').innerHTML = error_msg.message;
    }
  }

  function errorHandler(event) {
    document.querySelector('#status').innerHTML = event.target.responseText.message;
  }

  function abortHandler(event) {
    document.querySelector('#status').innerHTML = "Upload Aborted";
}

function delete_file(file) {
    fetch("/tab-files-delete", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({'delete_this':file})
    })
    .then(function(response) {
        console.log(response);
        get_tab_files();
    });
}

function change_events() {
    document.querySelectorAll('#upload-tab-table-body-files input').forEach(function(element) {
        removeEventListener('change',element);
        element.addEventListener('change', function() {
            save_tab_files();
        });
    });
}

function save_tab_files() {
    const files = document.querySelectorAll("#upload-tab-table-body-files tr");
    const data = {};
    const uploaded_files = {};
    let i = 0;
    files.forEach(function(element) {
        const name = element.dataset.file;
        const which_set = element.querySelector(`input[name="file-which[${i}]"]:checked`).value;
        uploaded_files[name] = {
            which_set: which_set,
        }
        i++;
    });
    data.uploaded_files = uploaded_files;
    console.log('data', data);
    fetch("/tab-files-save-config", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(data)
    })
    .then(function(response) {
        console.log(response);
        // get_tab_files();
    });
}

const process_now_update_until_finished = async () => {
    const process_button = document.querySelector('.tab-files-process-now');
    process_button.disabled = true;
    process_button.dataset.loading = true;
};

function file_status_color(status) {
    let status_color;
    switch (status) {
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
    return status_color;
}

function render_filter_setup_defaults(data) {
    // TODO XXX this should come from /tab-finetune-smart-filter-get
    return;
    if(sources_settings_modal) { return; }
    document.querySelector('#filter_gradcosine_threshold').value = data.filter_gradcosine_threshold;
    document.querySelector('#filter_loss_threshold').value = data.filter_loss_threshold;
    document.querySelector('#limit_test_files').value = data.limit_test_files;
    document.querySelector('#limit_time_seconds').value = data.limit_time_seconds;
    document.querySelector('#limit_train_files').value = data.limit_train_files;
}

function save_filter_setup() {
    const filter_gradcosine_threshold = document.querySelector('#filter_gradcosine_threshold').value;
    const filter_loss_threshold = document.querySelector('#filter_loss_threshold').value;
    // const limit_test_files = document.querySelector('#limit_test_files').value;
    const limit_time_seconds = document.querySelector('#limit_time_seconds').value;
    const limit_train_files = document.querySelector('#limit_train_files').value;
    let include_file_types = null;
    const file_types = document.querySelectorAll('.upload-tab-table-type-body tr.enabled-file input:checked');
    if(file_types.length > 0) {
        include_file_types = {};
        file_types.forEach(function(element) {
            include_file_types[element.dataset.name] = true;
        });
    }
    const force_include = document.querySelector('#force_include').value;
    const force_exclude = document.querySelector('#force_exclude').value;
    fetch("/tab-files-setup-filtering", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            filter_gradcosine_threshold: filter_gradcosine_threshold,
            filter_loss_threshold: filter_loss_threshold,
            limit_time_seconds: limit_time_seconds,
            limit_train_files: limit_train_files,
            include_file_types: include_file_types,
            force_include: force_include,
            force_exclude: force_exclude,
        })
    })
    .then(function(response) {
        console.log(response);
    });
}

export function init() {
    const run_filter_button = document.querySelector('.sources-run-button');
    run_filter_button.addEventListener('click', function() {
        save_filter_setup();
    });
    const tab_upload_file_submit = document.querySelector('.tab-upload-file-submit');
    tab_upload_file_submit.removeEventListener('click', upload_file());
    tab_upload_file_submit.addEventListener('click', function() {
        upload_file();
    });

    const tab_upload_url_submit = document.querySelector('.tab-upload-url-submit');
    tab_upload_url_submit.removeEventListener('click', upload_url());
    tab_upload_url_submit.addEventListener('click', function() {
        upload_url();
    });

    const tab_upload_git_submit = document.querySelector('.tab-upload-git-submit');
    tab_upload_git_submit.removeEventListener('click', upload_repo());
    tab_upload_git_submit.addEventListener('click', function() {
        upload_repo();
    });

    const process_button = document.querySelector('.tab-files-process-now');
    process_button.addEventListener('click', function() {
        fetch("/tab-files-process-now")
            .then(function(response) {
                process_now_update_until_finished();
            });
    });
    const file_modal = document.getElementById('upload-tab-files-modal');
    file_modal.addEventListener('show.bs.modal', function () {
        file_modal.querySelector('#tab-upload-file-input').value = '';
        file_modal.querySelector('.progress-bar').setAttribute('aria-valuenow', 0);
        file_modal.querySelector('.progress').classList.add('d-none');
        file_modal.querySelector('.tab-upload-file-submit').classList.remove('d-none');
        file_modal.querySelector('#status').innerHTML = '';
        file_modal.querySelector('#loaded_n_total').innerHTML = '';
    });

    const url_modal = document.getElementById('upload-tab-url-modal');
    url_modal.addEventListener('show.bs.modal', function () {
        url_modal.querySelector('#tab-upload-url-input').value = '';
        url_modal.querySelector('#status-url').innerHTML = '';
    });

    const settings_modal = document.getElementById('upload-tab-source-settings-modal');
    settings_modal.addEventListener('show.bs.modal', function () {
        sources_settings_modal = true;
    });

    const settings_modal_submit = document.querySelector('.tab-upload-source-settings-submit');
    settings_modal_submit.addEventListener('click', function() {
        const settings_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-tab-source-settings-modal'));
        settings_modal.hide();
    });

    const git_modal = document.getElementById('upload-tab-git-modal');
    git_modal.addEventListener('show.bs.modal', function () {
        get_ssh_keys();
        git_modal.querySelector('#tab-upload-git-input').value = '';
        git_modal.querySelector('#tab-upload-git-brach-input').value = '';
        git_modal.querySelector('#status-git').innerHTML = '';
    });

    const ssh_link = document.querySelector('.ssh-link');
    ssh_link.addEventListener('click', function(event) {
        event.preventDefault()
        const settings_tab = new bootstrap.Tab(document.querySelector('#settings-tab'));
        const ssh_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-tab-git-modal'));
        ssh_modal.hide();
        settings_tab.show();
        document.querySelector('.dropdown-menu').classList.remove('show');
    });
    let delete_modal_button = document.querySelector('.delete-modal-submit');
    delete_modal_button.addEventListener('click', function() {
        if(this.dataset.file && this.dataset.file !== '') {
            delete_file(this.dataset.file);
            this.dataset.file = "";
        }
        let delete_modal_instance = bootstrap.Modal.getOrCreateInstance(document.getElementById('delete-modal'));
        delete_modal_instance.hide();
    });
}

export function tab_switched_here() {
    get_tab_files();
}
