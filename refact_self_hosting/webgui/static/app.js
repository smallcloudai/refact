(function() {
    // Comming soon
    let comming_soon = document.querySelectorAll(".temp-disabled");
    function comming_soon_render() {
        comming_soon.forEach(function(element) {
            const info = element.parentNode.insertBefore(document.createElement("div"), element.nextSibling);
            info.classList.add("temp-info");
            info.innerHTML = "Coming soon";
            info.style.marginLeft = ((element.getBoundingClientRect().width / 2 ) - (info.getBoundingClientRect().width / 2 )) + "px";
            info.style.marginTop = ((element.getBoundingClientRect().height / 2 ) * -1 - (info.getBoundingClientRect().height / 2 )) + "px";
        });
    }

    function comming_soon_resize()  {
        comming_soon.forEach(function(element) {
            const info = element.nextSibling;
            info.style.marginLeft = ((element.getBoundingClientRect().width / 2 ) - (info.getBoundingClientRect().width / 2 )) + "px";
            info.style.marginTop = ((element.getBoundingClientRect().height / 2 ) * -1 - (info.getBoundingClientRect().height / 2 )) + "px";
        });
    }
    comming_soon_render();

    window.addEventListener("resize", function() {
        comming_soon_resize();
    });

    document.addEventListener('shown.bs.tab', function(e) {
        comming_soon_resize();
    });


    // Fetch files
    function get_tab_files() {
        fetch("/tab-files-get")
            .then(function(response) {
                return response.json();
            })
            .then(function(data) {
                console.log(data);
                render_tab_files(data);
            });
    }
    get_tab_files();

    function render_tab_files(data) {
        const files = document.getElementById("files");
        files.innerHTML = "";
        let i = 0;
        for(item in data.uploaded_files) {
            const row = document.createElement('tr');
            const name = document.createElement("td");
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
        document.querySelectorAll('#files .file-remove').forEach(function(element) {
            removeEventListener('click',element);
            element.addEventListener('click', function() {
                delete_file(this.getAttribute('data-file'));
            });
        });
    }

    const submit_file_button = document.querySelector('.submit-file');
    submit_file_button.addEventListener('click', function() {
        upload_file();
    });

    const submit_url_button = document.querySelector('.submit-url');
    submit_url_button.addEventListener('click', function() {
        upload_url();
    });

    const submit_git_button = document.querySelector('.submit-git');
    submit_git_button.addEventListener('click', function() {
        upload_repo();
    });

    const process_button = document.querySelector('.process-now');
    process_button.addEventListener('click', function() {
        process_now();
    });

    function process_now() {
        fetch("/tab-files-process-now")
            .then(function(response) {
                return response.json();
            })
            .then(function(data) {
                console.log(data);
                render_tab_files(data);
            });
    }

    function upload_url() {
        const fileInput = document.querySelector('#urlInput');
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
            document.querySelector('#fileInput').value = '';
            let url_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('urlModal'));
            url_modal.hide();
        })
        .catch(error => {
            document.querySelector('#status-url').innerHTML = error.message;
        });
    }

    function upload_repo() {
        const gitUrl = document.querySelector('#gitInput');
        const gitBranch = document.querySelector('#gitBranch');   
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
                throw new Error('Network response was not ok');
            }
            return response.text();
        })
        .then(data => {
            get_tab_files();
            document.querySelector('#gitInput').value = '';
            let git_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('gitModal'));
            git_modal.hide();
        })
        .catch(error => {
            document.querySelector('#status-git').innerHTML = error.message;
        });
    }

    function upload_file() {
        const fileInput = document.querySelector('#fileInput');
        if(fileInput.files.length === 0) {
            return;
        }
        var formdata = new FormData();
        formdata.append("file", fileInput.files[0]);
        document.querySelector('.progress').classList.toggle('d-none');
        document.querySelector('.submit-file').classList.toggle('d-none');
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
        setTimeout(() => {
            document.querySelector('.progress-bar').setAttribute('aria-valuenow', 0);
            document.querySelector('.progress').classList.toggle('d-none');
            document.querySelector('.submit-file').classList.toggle('d-none');
            get_tab_files();
            document.querySelector('#fileInput').value = '';
            let file_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('uploadModal'));
            file_modal.hide();
        }, 2000);
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
        document.querySelectorAll('#files input').forEach(function(element) {
            removeEventListener('change',element);
            element.addEventListener('change', function() {
                save_tab_files();
            });
        });
    }


    function save_tab_files() {
        const files = document.querySelectorAll("#files tr");
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
    finetune_data();

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
            if(i === 0) {
                document.querySelector('.fine-gfx').src = `data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+ip1sAAAAASUVORK5CYII=`;
                document.querySelector('.fine-gfx').style.height = '200px';
            }
            rows.forEach(function(row) {
                row.addEventListener('click', function() {
                    rows.forEach(function(row) {
                        row.classList.remove('table-primary');
                    });
                    this.classList.add('table-primary');
                    const run_id = this.dataset.run;
                    document.querySelector('.fine-gfx').src = `/tab-finetune-progress-svg/${run_id}`;
                    document.querySelector('.fine-gfx').style.height = '';
                    get_log(run_id);
                });
            });
            i++;
        });
    }

   function get_log(run_id) {
        console.log('get_log', run_id);
        fetch(`/tab-finetune-log/${run_id}`)
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            const logs_container = document.querySelector('.finetune-logs');
            logs_container.innerHTML = '';
            logs_container.innerHTML = data.log.join('<br>');
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
    render_time_dropdown();

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
})();