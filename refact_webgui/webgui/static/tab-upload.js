import { general_error } from './error.js';
import { init as init_upload_files_modal, switch_away as upload_files_modal_switch_away } from './components/modals/modal-upload-files.js'


let tab_files_data = null;
let show_scan_error = false;
let sort_type = 'filetype';
let sort_order = 'asc';
let sort_started = false;
let dont_disable_file_types = false;
let pname = "project1"

let sources_pane = null;
let filetypes_pane = null;
let projects_list = null;

// function do_starting_state() {
//     filetypes_pane.classList.add('pane-disabled');
//     if(!document.querySelector('.sources-run-button .spinner-border')) {
//         sources_run_button.innerHTML = `<span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span></i>Starting`;
//         sources_status.innerHTML = 'starting';
//     }
// }

function get_projects_list() {
    // fetch("/tab-project-list")
    //     .catch(function(error) {
    //         console.log('tab-project-list',error);
    //         general_error(error);
    //     })       
    //    .then(function(response) {
    //         return response.json();
    //     })
    //     .then(function(data) {
    //         projects_list = data;
    //     });
    projects_list = [
        {
            name: 'Project_1',
        },
        {
            name: 'Project_2',
        },
        {
            name: 'Project_3',
        },
    ];
    projects_dropdown();
    projects_list.reverse().forEach(function(project) {
        const list_item = document.createElement('li');
        const link = document.createElement('button');
        link.classList.add('dropdown-item');
        link.value = project.name;
        link.innerHTML = project.name;
        list_item.appendChild(link);
        document.querySelector('.projects-dropdown').prepend(list_item);
    });
    const start_project_button = document.querySelector('.start-project');
    start_project_button.addEventListener('click', start_new_project);
    const project_buttons = document.querySelectorAll('.projects-dropdown.dropdown-item');
    project_buttons.forEach(function(project_button) {
        project_button.addEventListener('click', show_project(project_button.value));
    });
}

function show_project() {

}

function start_new_project() {
    fetch("/tab-project-new")
    .catch(function(error) {
        console.log('tab-project-list',error);
        general_error(error);
    }) 
   .then(function(response) {
        return response.json();
    })
    .then(function(data) {
    });
}

function projects_dropdown() {
    const project_dropdown = document.querySelector('.main-tab-button[data-tab="upload"]').parentNode;
    project_dropdown.classList.add('dropdown');
    project_dropdown.innerHTML = '';
    const link = document.createElement('a');
    link.href = "#";
    link.classList.add('dropdown-toggle');
    link.classList.add('nav-link');
    link.setAttribute('data-bs-toggle', 'dropdown');
    link.setAttribute('role', 'button');
    link.setAttribute('aria-expanded', 'false');
    link.innerHTML = "Projects";
    project_dropdown.appendChild(link);
    
    const dropdown = document.createElement('ul');
    dropdown.classList.add('dropdown-menu');
    dropdown.classList.add('projects-dropdown');
    dropdown.setAttribute('aria-labelledby', 'navbarDropdown');
    dropdown.innerHTML = `<li><hr class="dropdown-divider"></li>
                        <li><button class="dropdown-item start-project">New Project &hellip;</button></li>`;
    project_dropdown.appendChild(dropdown);

    // <li class="nav-item dropdown">
    //       <a class="nav-link dropdown-toggle" href="#" id="navbarDropdown" role="button" data-bs-toggle="dropdown" aria-expanded="false">
    //         Dropdown
    //       </a>
    //       <ul class="dropdown-menu" aria-labelledby="navbarDropdown">
    //         <li><a class="dropdown-item" href="#">Action</a></li>
    //         <li><a class="dropdown-item" href="#">Another action</a></li>
    //         <li><hr class="dropdown-divider"></li>
    //         <li><a class="dropdown-item" href="#">Something else here</a></li>
    //       </ul>
    //     </li>
}

function get_tab_files() {
    fetch(`/tab-files-get/${pname}`)
        .then(function(response) {
            return response.json();
        })
        .catch(function(error) {
            console.log('tab-files-get',error);
            general_error(error);
        })
        .then(function(data) {
            // console.log('tab-files-get',data);
            tab_files_data = data;
            if(data.scan_error && data.scan_error.length > 0) {
                let scan_toast = document.querySelector('.upload-tab-scan-error-toast');
                const scan_error_toast = bootstrap.Toast.getOrCreateInstance(scan_toast);
                if(!show_scan_error) {
                    document.querySelector('.upload-tab-scan-error-toast .toast-body').innerHTML = data.scan_error;
                    scan_error_toast.show();
                    show_scan_error = true;
                }
            }
            if(!data.scan_status) {
                document.querySelector('.filetypes-pane').classList.add('pane-disabled');
            }
            // render_filter_progress(data.filtering_progress);
            if(dont_disable_file_types || data.scan_status && data.scan_status === 'finished') {
                document.querySelector('.filetypes-pane').classList.remove('pane-disabled');
            } else {
                document.querySelector('.filetypes-pane').classList.add('pane-disabled');
            }
            render_tab_files(data);
            render_filetypes(data.mime_types, data.filetypes);
            render_force_filetypes(data.filetypes);
            if (data.disable_ui) {
                sources_pane.classList.add('pane-disabled');
                filetypes_pane.classList.add('pane-disabled');
            } else {
                sources_pane.classList.remove('pane-disabled');
                filetypes_pane.classList.remove('pane-disabled');
            }
        });
}

function render_tab_files(data) {
    const files = document.getElementById("upload-tab-table-body-files");
    let i = 0;
    if(Object.keys(data.uploaded_files).length >= 0) {
        files.innerHTML = "";
    }
    if(Object.keys(data.uploaded_files).length === 0) {
        files.innerHTML = `<tr><td>No sources added.</td><td></td><td></td><td></td><td></td></tr>`;
    }
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
                } else if (current_status === 'failed') {
                    target_cell.innerHTML = `<span class="file-status badge rounded-pill ${status_color}">${current_status}</span><div class="source-popup"><pre>${item_object.message}</pre></div>`;
                    row.querySelector('.file-status').addEventListener('mouseover', function(event) {
                        event.target.nextElementSibling.style.display = 'block';
                        // null on reading style
                    });
                    row.querySelector('.file-status').addEventListener('mouseout', function(event) {
                        // null on reading s
                        event.target.nextElementSibling.style.display = 'none';
                    });
                } else {
                    target_cell.innerHTML = `<span class="file-status badge rounded-pill ${status_color}">${current_status}</span>`;
                }
                if (current_status == "working" || current_status == "starting") {
                    if(item_object.files === undefined) {
                        target_cell.innerHTML = `<span class="file-status badge rounded-pill ${status_color}">${current_status}</span>`;
                    } else {
                        target_cell.innerHTML = `<span class="file-status badge rounded-pill ${status_color}">${current_status}</span><span>${item_object.files} files</span>`;
                    }
                    any_working = true;
                }
                break;
            }
        }
    }

    const process_button = document.querySelector('.tab-files-process-now');
    if (any_working) {
        // let process_button_text = "Scanning...";
        // process_button.innerHTML = '<div class="upload-spinner spinner-border spinner-border-sm" role="status"></div>' + process_button_text;
    } else {
        if (process_button.dataset.loading) {
            process_button.dataset.loading = false;
            process_button.disabled = false;
        }
        // process_button.innerHTML = "Scan sources";
    }
}

function get_ssh_keys() {
    fetch("/tab-settings-get-all-ssh-keys")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            console.log('get-all-ssh-keys',data);
        })
       .catch(function(error) {
            console.log('get-all-ssh-keys',error);
            general_error(error);
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

let force_include_exclude_is_changed = false;
function render_force_filetypes(data) {
    if(force_include_exclude_is_changed) { return; }
    const force_include = document.querySelector('#force_include');
    const force_exclude = document.querySelector('#force_exclude');

    let force_include_val = data.force_include;
    if (force_include_val === undefined) {
        force_include_val = ""
    }
    force_include.value = force_include_val;
    let force_exclude_val = data.force_exclude;
    if (force_exclude_val === undefined) {
        force_exclude_val = ""
    }
    force_exclude.value = force_exclude_val;
}


function render_filetypes(mimetypes, filetypes) {
    if(mimetypes && mimetypes.length > 0) {
        const table_body = document.querySelector('.upload-tab-table-type-body');
        table_body.innerHTML = '';
        let i = 0;
        const sorted_mime_types = sort_filetypes(mimetypes);
        sorted_mime_types.forEach((item) => {
            const row = document.createElement('tr');
            let checkbox_checked = `checked`;
            let file_name = `<label for="file-list${i}" style="font-weight: bold">${item.file_type}</label>`;
            if (!item.trusted_language) {
                file_name = `<label for="file-list${i}" title="This file type may not yield optimal results for fine-tuning">${item.file_type}</label>`;
            }
            if (item.suitable_to_train) {
                row.classList.add('enabled-file');
            } else {
                row.classList.add('opacity-50');
                row.classList.add('disbled');
                checkbox_checked = `disabled`;
            }
            if (filetypes && filetypes["filetypes_finetune"][item.file_type] !== true) {
                checkbox_checked = `unchecked`;
            }
            let file_checkbox = `<input id="file-list${i}" data-name="${item.file_type}" class="form-check-input" type="checkbox" value="${item.count}" ${checkbox_checked}>`;
            row.innerHTML = `<td>${file_checkbox}</td><td>${file_name}</td><td>${item.count}</td>`;
            table_body.appendChild(row);
            i++;
        });
        render_stats();
        watch_filetypes();
    }
}

function sort_filetypes(sorted_data) {
    if(!sort_started) { return sorted_data; }
    if(sort_type === 'filetype') {
        if (sort_order === 'asc') {
            sorted_data.sort((a, b) => a.file_type.localeCompare(b.file_type));
        } else {
            sorted_data.sort((a, b) => b.file_type.localeCompare(a.file_type));
        }
    }
    if(sort_type === 'count') {
        if (sort_order === 'asc') {
            sorted_data.sort(function (a, b) {
                return a.count - b.count;
            });
        } else {
            sorted_data.sort(function (a, b) {
                return b.count - a.count;
            });
        }
    }
    return sorted_data;
  }


function watch_filetypes() {
    const file_types = document.querySelectorAll('.upload-tab-table-type-body input');
    if(file_types.length > 0) {
        file_types.forEach(function(element) {
            element.addEventListener('change', function() {
                dont_disable_file_types = true;
                save_filter_setup();
            });
        });
    }
}

function save_filter_setup() {
    const include_file_types = {};
    const checked_types = document.querySelectorAll('.upload-tab-table-type-body tr input[type="checkbox"]:checked')
    checked_types.forEach(function(element) {
        include_file_types[element.dataset.name] = true;
    });
    const force_include = document.querySelector('#force_include').value;
    const force_exclude = document.querySelector('#force_exclude').value;
    fetch(`/tab-files-filetypes-setup/${pname}`, {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            filetypes_finetune: include_file_types,
            force_include: force_include,
            force_exclude: force_exclude,
        })
    })
    .then(response => {
        if (!response.ok) {
            throw new Error(response.statusText);
        }
    })
   .catch(error => {
        console.log('tab-files-filetypes-setup',error);
        general_error(error);
    });
}

function render_stats() {
    const stats_finetune = document.querySelector('.sources-stats-finetune');
    // const stats_db = document.querySelector('.sources-stats-db');
    if (tab_files_data.filestats_scan_finetune && typeof tab_files_data.filestats_scan_finetune === 'object') {
        if(Object.keys(tab_files_data.filestats_scan_finetune).length > 0) {
            stats_finetune.style.display = 'flex';
            stats_finetune.style.whiteSpace = 'nowrap';
            const fine_accepted = document.querySelector('.sources-stats-fine-accepted span');
            fine_accepted.innerHTML = `${tab_files_data.filestats_scan_finetune.accepted} <a target="_blank" class="sources-stats-fine-accepted" href="/tab-files-log?accepted_or_rejected=accepted">Full List</a>`;
            const fine_rejected = document.querySelector('.sources-stats-fine-rejected span');
            fine_rejected.innerHTML = `${tab_files_data.filestats_scan_finetune.rejected} <a target="_blank" class="sources-stats-fine-rejected" href="/tab-files-log?accepted_or_rejected=rejected">Full List</a>`;
        }
    }
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

    fetch(`/tab-files-repo-upload/${pname}`, {
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
        process_now_update_until_finished();
        gitUrl.value = '';
        gitBranch.value = '';
        let git_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-tab-git-modal'));
        git_modal.hide();
    })
    .catch(error => {
        document.querySelector('#status-git').innerHTML = error.message;
    });
}

function delete_file(file) {
    fetch(`/tab-files-delete/${pname}`, {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({'delete_this':file})
    })
    .catch(function(error) {
        console.log('tab-files-delete',error);
        general_error(error);
    })
    .then(function(response) {
        process_now_update_until_finished();
        console.log(response);
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

    fetch(`/tab-files-save-config/${pname}`, {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(data)
    })
    .catch(function(error) {
        console.log('tab-files-save-config',error);
        general_error(error);
    })
    .then(function(response) {
        if(response.ok) {
            get_tab_files();
        }
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

export async function init(general_error) {
    let req = await fetch('/tab-upload.html');
    document.querySelector('#upload').innerHTML = await req.text();
    get_projects_list();
    sources_pane = document.querySelector('.sources-pane');
    filetypes_pane = document.querySelector('.filetypes-pane');

    const tab_upload_git_submit = document.querySelector('.tab-upload-git-submit');
    tab_upload_git_submit.removeEventListener('click', upload_repo());
    tab_upload_git_submit.addEventListener('click', function() {
        upload_repo();
    });

    const process_button = document.querySelector('.tab-files-process-now');
    process_button.addEventListener('click', function() {
        fetch(`/tab-files-process-now/${pname}?git_pull=1`)
        .then(function(response) {
            process_now_update_until_finished();
        });
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
        const ssh_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-tab-git-modal'));
        ssh_modal.hide();

        document.querySelector('#upload').classList.remove('main-active');
        document.querySelector('#settings').classList.add('main-active');
        document.querySelectorAll('.nav-link').forEach(function(link) {
            link.classList.remove('main-active');
        });
    });

    let delete_modal_button = document.querySelector('.delete-modal-submit');
    delete_modal_button.addEventListener('click', function() {
        if(this.dataset.file && this.dataset.file !== '') {
            delete_file(this.dataset.file);
            this.dataset.file = "";
        }
        let delete_modal_instance = bootstrap.Modal.getOrCreateInstance(document.getElementById('delete-modal'));
        delete_modal_instance.hide();
        process_now_update_until_finished();
    });

    const force_include = document.getElementById('force_include');
    force_include.addEventListener("input", function () {
        force_include_exclude_is_changed = true;
    })
    force_include.addEventListener("change", function () {
        dont_disable_file_types = true;
        save_filter_setup();
        force_include_exclude_is_changed = false;
    })
    const force_exclude = document.getElementById('force_exclude');
    force_exclude.addEventListener("input", function () {
        force_include_exclude_is_changed = true;
    })
    force_exclude.addEventListener("change", function () {
        dont_disable_file_types = true;
        save_filter_setup();
        force_include_exclude_is_changed = false;
    })

    const mime_sort = document.querySelector('.upload-tab-table-mime-sort');
    mime_sort.addEventListener('click', (event) => {
        sort_type = 'filetype';
        sort_started = true;
        if(event.target.classList.contains('bi-caret-up-fill')) {
            sort_order = 'desc';
            event.target.classList.remove('bi-caret-up-fill');
            event.target.classList.add('bi-caret-down-fill');
        } else {
            sort_order = 'asc';
            event.target.classList.remove('bi-caret-down-fill');
            event.target.classList.add('bi-caret-up-fill');
        }
    });

    const count_sort = document.querySelector('.upload-tab-table-count-sort');
    count_sort.addEventListener('click', (event) => {
        sort_type = 'count';
        sort_started = true;
        if(event.target.classList.contains('bi-caret-up-fill')) {
            sort_order = 'desc';
            event.target.classList.remove('bi-caret-up-fill');
            event.target.classList.add('bi-caret-down-fill');
        } else {
            sort_order = 'asc';
            event.target.classList.remove('bi-caret-down-fill');
            event.target.classList.add('bi-caret-up-fill');
        }
    });

    const checkall_button = document.querySelector('.filetypes-checkall');
    checkall_button.addEventListener('click', function() {
        const file_inputs = document.querySelectorAll('.upload-tab-table-type-body .enabled-file input[type="checkbox"]');
        for (let i = 0; i < file_inputs.length; i++) {
            file_inputs[i].checked = true;
        }
        save_filter_setup();
    });

    const uncheckall_button = document.querySelector('.filetypes-uncheckall');
    uncheckall_button.addEventListener('click', function() {
        const file_inputs = document.querySelectorAll('.upload-tab-table-type-body .enabled-file input[type="checkbox"]');
        for (let i = 0; i < file_inputs.length; i++) {
            file_inputs[i].checked = false;
        }
        save_filter_setup();
    });

    const proceed_finetune_button = document.querySelector('.proceed-finetune');
    proceed_finetune_button.addEventListener('click', function() {
        document.querySelector('#upload').classList.remove('main-active');
        document.querySelector('#finetune').classList.add('main-active');
        document.querySelectorAll('.nav-link').forEach(function(link) {
            link.classList.remove('main-active');
        });
        let nav_link = document.querySelector('[data-tab="finetune"]');
        nav_link.classList.add('main-active');
    });
}

export function tab_switched_here() {
    get_tab_files();
    const files_modal = document.querySelector('#upload-files-modal');
    init_upload_files_modal(
        files_modal,
        document.querySelector('#open-upload-files-modal'),
        'Upload Files',
        'input',
        '/tab-files-upload-url', `/tab-files-upload/${pname}`,
        "Wrapping up. Please wait...",
        "https://yourserver.com/file.zip",
        "You can upload .zip, .tar.gz, .tar.bz2 archives, or an individual file such as \"my_program.py\""
    );
}

export function tab_switched_away() {
    dont_disable_file_types = false;
    upload_files_modal_switch_away(document.querySelector('#upload-files-modal'));
}

export function tab_update_each_couple_of_seconds() {
    get_tab_files();
}
