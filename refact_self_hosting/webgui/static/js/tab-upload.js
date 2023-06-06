function get_tab_files() {
    fetch("/tab-files-get")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            console.log('tab-files-get',data);
            render_tab_files(data);
        });
}

function render_tab_files(data) {
    const files = document.getElementById("upload-tab-table-body-files");
    files.innerHTML = "";
    let i = 0;
    for(let item in data.uploaded_files) {
        const row = document.createElement('tr');
        row.setAttribute('data-file', item);
        const name = document.createElement("td");
        const status = document.createElement("td");
        const train = document.createElement("td");
        const test = document.createElement("td");
        const context = document.createElement("td");
        const delete_file = document.createElement("td");
        name.innerHTML = item;
        const context_input = data.uploaded_files[item].to_db ? `<input class="form-check-input" name="file-context" type="checkbox" checked="checked">` : `<input class="form-check-input" name="file-context" type="checkbox">`
        const which_set = data.uploaded_files[item].which_set;
        if(which_set === "train") {
            train.innerHTML = `<input class="form-check-input" class="file-radio" value="train" name="file-which[${i}]" type="radio" checked="checked">`;
            test.innerHTML = `<input class="form-check-input" class="file-radio" value="test" name="file-which[${i}]" type="radio">`;
        }
        if(which_set === "test") {
            train.innerHTML = `<input class="form-check-input" class="file-radio" value="train" name="file-which[${i}]" type="radio">`;
            test.innerHTML = `<input class="form-check-input" class="file-radio" value="test" name="file-which[${i}]" type="radio" checked="checked">`;
        }
        context.innerHTML = context_input;
        // delete_file.innerHTML = `<button data-file="${item}" type="button" class="btn file-remove btn-danger btn-sm"><i class="bi bi-trash3-fill"></i></button>`;
        delete_file.innerHTML = `<div class="btn-group dropend"><button type="button" class="btn btn-danger btn-sm dropdown-toggle" data-bs-toggle="dropdown" aria-expanded="false"><i class="bi bi-trash3-fill"></i></button><ul class="dropdown-menu"><li><span class="file-remove dropdown-item btn btn-sm" data-file="${item}">Delete file</a></span></ul></div>`;
        row.appendChild(name);
        row.appendChild(status);
        row.appendChild(train);
        row.appendChild(test);
        row.appendChild(context);
        row.appendChild(delete_file);
        files.appendChild(row);
        i++;
    }
    change_events();
    delete_events();
}

function delete_events() {
    document.querySelectorAll('#upload-tab-table-body-files .file-remove').forEach(function(element) {
        removeEventListener('click',element);
        element.addEventListener('click', function() {
            delete_file(this.getAttribute('data-file'));
        });
    });
}

function upload_url() {
    const fileInput = document.querySelector('#tab-upload-url-input');
    if(!fileInput || fileInput.value == '') {
        return;
    }
    console.log('fileInput.value',fileInput.value);
    const formData = {
        'url' : fileInput.value
    }

    fetch('/tab-files-upload-url', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(formData)
    })
    .then(response => {
        if (!response.ok) {
            throw new Error('Network response was not ok');
        }
        return response.text();
    })
    .then(data => {
        get_tab_files();
        fileInput.value = '';
        let url_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-tab-url-modal'));
        url_modal.hide();
    })
    .catch(error => {
        document.querySelector('#status-url').innerHTML = error.message;
    });
}

function upload_repo() {
    const gitUrl = document.querySelector('#tab-upload-git-input');
    const gitBranch = document.querySelector('#tab-upload-git-brach-input');
    if(!gitUrl || gitUrl.value == '') {
        return;
    }
    const formData = {
        'url' : gitUrl.value
    }
    if(gitBranch.value && gitBranch.value !== '') {
        formData['branch'] = gitBranch.value;
    }

    fetch('/tab-repo-upload', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(formData)
    })
    .then(response => {
        if (!response.ok) {
            // document.querySelector('#status-git').innerHTML = response.message;
            throw new Error('Network response was not ok');
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
        // document.querySelector('.progress-bar').setAttribute('aria-valuenow', 0);
        // document.querySelector('.progress').classList.toggle('d-none');
        // document.querySelector('.tab-upload-file-submit').classList.toggle('d-none');
        console.log(event.target.responseText);
        // document.querySelector('#status').innerHTML = event.target.responseText.message;
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
        body: JSON.stringify(file)
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
        const name = element.querySelector('td').innerHTML;
        const context = element.querySelector('input[name="file-context"]').checked;
        const which_set = element.querySelector(`input[name="file-which[${i}]"]:checked`).value;
        uploaded_files[name] = {
            which_set: which_set,
            to_db: context
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
        get_tab_files();
    });
}

const process_now_update_until_finished = async () => {
    const table = document.querySelector('#upload-tab-table-body-files');
    const process_button = document.querySelector('.tab-files-process-now');
    process_button.disabled = true;
    const process_button_text = process_button.innerHTML;
    process_button.innerHTML = '<div class="upload-spinner spinner-border spinner-border-sm" role="status"></div>' + process_button_text;
    while (true) {
      try {
        const response = await fetch('/tab-files-get');
        let data = await response.json();

        for (const [item,item_object] of Object.entries(data.uploaded_files)) {
            const rows = table.querySelectorAll('tr');
            for (const row of rows) {
                const row_file_name = row.getAttribute('data-file');
                if (row_file_name === item) {
                    const target_cell = row.querySelector('td:nth-child(2)');
                    const current_status = item_object.status;
                    const status_color = file_status_color(current_status);
                    let popup_data = `Status: ${item_object.status}`;
                    if(item_object.generated) {
                        popup_data += ` \nGenerated: ${item_object.generated}`;
                    }
                    if(item_object.good) {
                        popup_data += ` \nGood: ${item_object.good}`;
                    }
                    if(item_object.too_large) {
                        popup_data += ` \nToo Large: ${item_object.too_large}`;
                    }
                    if(item_object.vendored) {
                        popup_data += ` \nVendored: ${item_object.vendored}`;
                    }
                    target_cell.innerHTML = `<span data-bs-toggle="tooltip" data-bs-placement="top" data-bs-custom-class="custom-tooltip" data-bs-title="${popup_data}" class="file-status badge rounded-pill ${status_color}">${current_status}</span>`;
                    break;
                }
            }
        }

        if (data.finished === true) {
            process_button.disabled = false;
            process_button.innerHTML = process_button_text;
            const tooltipTriggerList = document.querySelectorAll('[data-bs-toggle="tooltip"]');
            const tooltipList = [...tooltipTriggerList].map(tooltipTriggerEl => new bootstrap.Tooltip(tooltipTriggerEl));
            break;
          }
      } catch (error) {
        console.error(error);
      }

      await new Promise(resolve => setTimeout(resolve, 1000));
    }
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
            status_color = `bg-light text-dark`;
            break;
        case 'failed':
            status_color = `bg-danger`;
            break;
    }
    return status_color;
}

function updateTest() {
    setTimeout(function() {
        test.finished = true;
        test.finished_ts = new Date().toISOString();
        test.uploaded_files = test.uploaded_files;
        test.files_after_dedup = test.files_after_dedup + 33;
        test.files_before_dedup = test.files_before_dedup + 55;
    }, 10000);
}

export function init() {
    get_tab_files();
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
}