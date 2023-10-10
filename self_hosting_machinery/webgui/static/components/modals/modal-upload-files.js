
let modal_html = `
<div class="modal fade" id="updlg-modal" tabindex="-1" role="dialog" aria-labelledby="updlg-modal" aria-hidden="true">
  <div class="modal-dialog modal-dialog-centered" role="document">
    <div class="modal-content">
      <div class="modal-header">
        <h5 class="modal-title" id="updlg-modal-label">Default Label</h5>
      </div>
      <div class="modal-body">
        <!-- Nav tabs -->
        <ul class="nav nav-tabs" id="updlg-nav-tabs" role="tablist" style="margin-bottom: 15px">
          <li class="nav-item">
            <button class="updlg-nav nav-link main-tab-button" id="updlg-nav-tab-link" data-method="link" data-toggle="updlg-link-div">Upload by Link</button>
          </li>
          <li class="nav-item">
            <button class="updlg-nav nav-link main-tab-button" id="updlg-nav-tab-input"  data-method="input" data-toggle="updlg-input-div">Choose file</button>
          </li>
        </ul>
        <!-- Tab panes -->
        <div class="tab-content">
          <div class="mb-3 form-text ssh-info"></div>
          <div class="updlg-pane-modal tab-pane fade" id="updlg-link-div" role="tabpanel" aria-labelledby="updlg-link-div">
            <div class="form-group">
              <input type="text" class="form-control" id="updlg-link" placeholder="Enter file URL">
            </div>
          </div>

          <div class="updlg-pane-modal tab-pane fade" id="updlg-input-div" role="tabpanel" aria-labelledby="updlg-input-div">
            <div class="form-group">
              <div class="mb-3">
                <input type="file" id="updlg-input" class="form-control">
              </div>
            </div>
          </div>
          <div id="updlg-file-upload-progress" class="progress d-none">
          <div id="updlg-file-upload-progress-bar" class="progress-bar progress-bar-animated" role="progressbar" aria-valuenow="0" aria-valuemin="0" aria-valuemax="100"></div>
          </div>
          <div id="updlg-loaded_n_total" class="uploaded-total"></div>
          <div>
            <span id="updlg-100-spinner" class="spinner-border spinner-border-sm" role="status" aria-hidden="true" hidden></span>
            <span id="updlg-status" class="uploaded-status sr-only"></span>
        </div>
        </div>
      </div>
      <div class="modal-footer" style="margin-top: 15px">
        <button type="button" class="btn btn-secondary" id="updlg-modal-close" data-bs-dismiss="modal">Close</button>
        <button type="button" class="btn btn-primary" id="updlg-modal-submit">Submit</button>
      </div>
    </div>
  </div>
</div>
`

let gl_open_on_click_el;
let gl_open_on_click_el_default_html;
let gl_modal;

function make_modal_backdrop_static() {
    gl_modal._config.backdrop = 'static';
}

// Function to make the modal backdrop responsive to outside clicks
function make_modal_backdrop_responsive() {
    gl_modal._config.backdrop = 'true';
}


export async function init(
    insert_in_el,
    open_on_click_el,
    modal_label,
    default_tab,
    submit_link_endpoint,
    submit_input_endpoint,
    text_on_progress_done,
    link_placeholder,
    input_help_text
) {
    gl_open_on_click_el = open_on_click_el;
    gl_open_on_click_el_default_html = open_on_click_el.innerHTML;
    insert_in_el.innerHTML = modal_html;

    gl_modal = new bootstrap.Modal(document.getElementById('updlg-modal'));
    insert_in_el.querySelector('#updlg-modal-label').innerText = modal_label;

    if (default_tab === 'link') {
        insert_in_el.querySelector('#updlg-nav-tab-link').classList.add('active', 'main-active')
        insert_in_el.querySelector('#updlg-link-div').classList.add('show', 'active')
    } else if (default_tab === 'input') {
        insert_in_el.querySelector('#updlg-nav-tab-input').classList.add('active','main-active')
        insert_in_el.querySelector('#updlg-input-div').classList.add('show', 'active')
    } else {
        console.log(`default tab ${default_tab} is not implemented!`);
    }
    if (link_placeholder) {
        insert_in_el.querySelector('#updlg-link').placeholder = link_placeholder;
    }
    if (input_help_text) {
        insert_in_el.querySelector('.ssh-info').innerText = input_help_text;
    }

    open_on_click_el.addEventListener('click', () => {
        show_modal();
    });

    add_nav_btn_click_handlers();
    add_submit_handler(submit_link_endpoint, submit_input_endpoint, text_on_progress_done);
}


function add_nav_btn_click_handlers() {
    const file_modal = document.getElementById('updlg-modal');

    const btns_nav_upload_files = file_modal.querySelectorAll('button.updlg-nav')
    const panes_upload_files = file_modal.querySelectorAll('.updlg-pane-modal');
    btns_nav_upload_files.forEach(
        el => el.addEventListener('click', () => {
            if (!el.classList.contains('active')) {
                btns_nav_upload_files.forEach(el => el.classList.remove('active', 'main-active'));
                el.classList.add('active', 'main-active');
                panes_upload_files.forEach(el => el.classList.remove('show', 'active'));
                file_modal.querySelector(`#${el.dataset.toggle}`).classList.add('show', 'active')
            }
        })
    );
}


function add_submit_handler(submit_link_endpoint, submit_input_endpoint, text_on_progress_done) {
    const file_modal = document.getElementById('updlg-modal');
    function get_upload_method() {
        const btns = file_modal.querySelectorAll('button.updlg-nav')
        for (const btn of btns) {
            if (btn.classList.contains('active')) {
                return btn.dataset.method;
            }
        }
    }

    const upload_files_submit = file_modal.querySelector('#updlg-modal-submit')

    upload_files_submit.addEventListener('click', () => {
        const upload_method = get_upload_method();
        if (upload_method === 'link') {
            upload_url(submit_link_endpoint);
        }
        else if (upload_method === 'input') {
            upload_file(submit_input_endpoint, text_on_progress_done);
        } else {
            console.log(`upload method ${upload_method} is not implemented!`);
        }
    });
}


function reset_modal_fields() {
    const file_modal = document.getElementById('updlg-modal');

    gl_open_on_click_el.innerHTML = gl_open_on_click_el_default_html;
    file_modal.querySelector('#updlg-input').value = '';
    file_modal.querySelector('#updlg-modal-submit').disabled = false;
    file_modal.querySelector('#updlg-nav-tab-link').disabled = false;
    file_modal.querySelector('#updlg-link').disabled = false;
    file_modal.querySelector('#updlg-input').disabled = false;
    file_modal.querySelector('#updlg-file-upload-progress-bar').setAttribute('aria-valuenow', '0');
    file_modal.querySelector('#updlg-file-upload-progress').classList.add('d-none');
    file_modal.querySelector('#updlg-loaded_n_total').innerHTML = "";
    file_modal.querySelector('#updlg-status').innerHTML = "";
    file_modal.querySelector('#updlg-link').value = "";
    file_modal.querySelector('#updlg-100-spinner').hidden = true;
    file_modal.querySelector('#updlg-nav-tab-input').disabled = false;
    make_modal_backdrop_responsive();
    file_modal.querySelector('#updlg-modal-close').disabled = false;
}

function hide_modal() {
    bootstrap.Modal.getOrCreateInstance(document.getElementById('updlg-modal')).hide();
}

function show_modal() {
    bootstrap.Modal.getOrCreateInstance(document.getElementById('updlg-modal')).show();
}


function prepare_for_upload(upload_method) {
    const file_modal = document.getElementById('updlg-modal');
    const process_button = file_modal.querySelector('#updlg-modal-submit');

    file_modal.querySelector('#updlg-modal-close').disabled = true;
    process_button.disabled = true;
    process_button.dataset.loading = 'true';
    make_modal_backdrop_static();

    if (gl_open_on_click_el.innerHTML === gl_open_on_click_el_default_html) {
        gl_open_on_click_el.innerHTML = `<span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span> Uploading`;
    }

    if (upload_method === 'link') {
        file_modal.querySelector('#updlg-nav-tab-input').disabled = true;
        file_modal.querySelector('#updlg-link').disabled = true;
        file_modal.querySelector('#updlg-100-spinner').hidden = false;
        file_modal.querySelector('#updlg-status').innerText = 'Uploading file. Please wait...'
    } else if (upload_method === 'input') {
        file_modal.querySelector('#updlg-nav-tab-link').disabled = true;
        file_modal.querySelector('#updlg-input').disabled = true;
    }
}


function upload_file(endpoint, text_on_progress_done) {
    const file_modal = document.getElementById('updlg-modal');
    const file_input = file_modal.querySelector('#updlg-input');
    const file_upload_progress = file_modal.querySelector('#updlg-file-upload-progress');
    const progress_bar = file_modal.querySelector('#updlg-file-upload-progress-bar');
    const upload_files_status = file_modal.querySelector('#updlg-status');

    function progress_handler(event) {
        prepare_for_upload('input');
        file_modal.querySelector('#updlg-loaded_n_total').innerText = "Uploaded " + event.loaded + " bytes of " + event.total;
        let percent = (event.loaded / event.total) * 100;
        progress_bar.setAttribute('aria-valuenow', Math.round(percent).toString());
        progress_bar.style.width = Math.round(percent).toString() + "%";
        upload_files_status.innerText = Math.round(percent).toString() + "% uploaded... please wait";
        if (Math.round(percent) >= 100) {
            upload_files_status.innerText = text_on_progress_done;
            file_modal.querySelector('#updlg-100-spinner').hidden = false;
        }
    }

    function complete_handler(event) {
        upload_files_status.innerText = event.target.responseText;

        if(event.target.status === 200) {
            setTimeout(() => {
                reset_modal_fields();
                hide_modal();
            }, 500);
        } else {
            let error_msg = JSON.parse(event.target.responseText);
            reset_modal_fields();
            upload_files_status.innerText = error_msg.detail;
        }
    }

    function error_handler(event) {
        upload_files_status.innerText = event.target.responseText.message;
    }

    function abort_handler() {
        upload_files_status.innerText = "Upload Aborted";
    }

    if (file_input.files.length === 0) {
        return;
    }
    let formdata = new FormData();
    formdata.append("file", file_input.files[0]);
    file_upload_progress.classList.toggle('d-none');
    let ajax = new XMLHttpRequest();
    ajax.upload.addEventListener("progress", progress_handler, false);
    ajax.addEventListener("load", complete_handler, false);
    ajax.addEventListener("error", error_handler, false);
    ajax.addEventListener("abort", abort_handler, false);
    ajax.open("POST", endpoint);
    ajax.send(formdata);
}


function upload_url(endpoint) {
    const file_modal = document.getElementById('updlg-modal');

    function handle_invalid_url() {
        const error = new Error('Invalid URL');
        file_modal.querySelector('#updlg-status').innerText = error.message;
    }

    const file_input = file_modal.querySelector('#updlg-link');
    if (!file_input || file_input.value === '') {
        return;
    }

    const url_regex = /^(ftp|http|https):\/\/[^ "]+$/;
    const is_url = url_regex.test(file_input.value);
    if (!is_url) {
        handle_invalid_url();
        return;
    }

    let formData = {
        'url': file_input.value
    };

    prepare_for_upload('link')
    fetch(endpoint, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(formData)
    })
        .then(
            response => {
                if (!response.ok) {
                    return response.json()
                        .then((response) => {
                            throw new Error(response['detail']);
                        });
                }
                reset_modal_fields();
                return response.json();
        })
        .then(
            () => {
                reset_modal_fields();
                hide_modal();
            })
        .catch(
            error => {
                reset_modal_fields();
                file_modal.querySelector('#updlg-status').innerText = error.message;
            });
}
