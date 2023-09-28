
let modal_html = `
<div class="modal fade" id="updlg-modal-upload-files" tabindex="-1" role="dialog" aria-labelledby="updlg-modal-upload-files" aria-hidden="true">
  <div class="modal-dialog modal-dialog-centered" role="document">
    <div class="modal-content">
      <div class="modal-header">
        <h5 class="modal-title" id="updlg-upload-modal-label">Default Label</h5>
      </div>
      <div class="modal-body">
        <!-- Nav tabs -->
        <ul class="nav nav-tabs" id="updlg-nav-upload-files-tabs" role="tablist" style="margin-bottom: 15px">
          <li class="nav-item">
            <button class="updlg-nav-upload-files nav-link main-tab-button" id="updlg-nav-upload-files-tab-link" data-method="link" data-toggle="updlg-tab-upload-files-link-div">Upload by Link</button>
          </li>
          <li class="nav-item">
            <button class="updlg-nav-upload-files nav-link main-tab-button" id="updlg-nav-upload-files-tab-input"  data-method="input" data-toggle="updlg-tab-upload-files-input-div">Choose file</button>
          </li>
        </ul>
        <!-- Tab panes -->
        <div class="tab-content">
          <div class="mb-3 form-text ssh-info"></div>
          <div class="updlg-pane-upload-files-modal tab-pane fade" id="updlg-tab-upload-files-link-div" role="tabpanel" aria-labelledby="updlg-tab-upload-files-link-div">
            <div class="form-group">
              <input type="text" class="form-control" id="updlg-upload-files-link" placeholder="Enter file URL">
              <div class="form-check">
                <input class="form-check-input" type="checkbox" value="" id="updlg-check-force-filename">
                <label class="form-check-label form-text ssh-info" for="updlg-check-force-filename">Set filename manually</label>
              </div>
              <input type="text" class="form-control" id="updlg-force-filename" placeholder="your_filename.zip" hidden>
            </div>
          </div>

          <div class="updlg-pane-upload-files-modal tab-pane fade" id="updlg-tab-upload-files-input-div" role="tabpanel" aria-labelledby="updlg-tab-upload-files-input-div">
            <div class="form-group">
              <div class="mb-3">
                <input type="file" id="updlg-upload-files-input" class="form-control">
              </div>
            </div>
          </div>
          <div id="updlg-file-upload-progress" class="progress d-none">
          <div id="updlg-file-upload-progress-bar" class="progress-bar progress-bar-animated" role="progressbar" aria-valuenow="0" aria-valuemin="0" aria-valuemax="100"></div>
          </div>
          <div id="updlg-loaded_n_total" class="uploaded-total"></div>
          <div>
            <span id="updlg-upload-files-100-spinner" class="spinner-border spinner-border-sm" role="status" aria-hidden="true" hidden></span>
            <span id="updlg-upload-files-status" class="uploaded-status sr-only"></span>
        </div>
        </div>
      </div>
      <div class="modal-footer" style="margin-top: 15px">
        <button type="button" class="btn btn-secondary" id="updlg-upload-files-modal-close" data-bs-dismiss="modal">Close</button>
        <button type="button" class="btn btn-primary" id="updlg-upload-files-modal-submit">Submit</button>
      </div>
    </div>
  </div>
</div>
`

let gl_open_on_click_el;
let gl_open_on_click_el_default_html;
let gl_modal;

function makeModalBackdropStatic() {
    gl_modal._config.backdrop = 'static';
}

// Function to make the modal backdrop responsive to outside clicks
function makeModalBackdropResponsive() {
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

    gl_modal = new bootstrap.Modal(document.getElementById('updlg-modal-upload-files'));
    insert_in_el.querySelector('#updlg-upload-modal-label').innerHTML = modal_label;

    if (default_tab === 'link') {
        insert_in_el.querySelector('#updlg-nav-upload-files-tab-link').classList.add('active', 'main-active')
        insert_in_el.querySelector('#updlg-tab-upload-files-link-div').classList.add('show', 'active')
    } else if (default_tab === 'input') {
        insert_in_el.querySelector('#updlg-nav-upload-files-tab-input').classList.add('active','main-active')
        insert_in_el.querySelector('#updlg-tab-upload-files-input-div').classList.add('show', 'active')
    } else {
        console.log(`default tab ${default_tab} is not implemented!`);
    }
    if (link_placeholder) {
        insert_in_el.querySelector('#updlg-upload-files-link').placeholder = link_placeholder;
    }
    if (input_help_text) {
        insert_in_el.querySelector('.ssh-info').innerHTML = input_help_text;
    }
    const modal_events = new UploadFilesModalEvents(
        submit_link_endpoint,
        submit_input_endpoint,
        text_on_progress_done
    );

    open_on_click_el.addEventListener('click', () => {
        modal_events.show_modal();
    });
}

class UploadFilesModalEvents {
    constructor(
        submit_link_endpoint,
        submit_input_endpoint,
        text_on_progress_done,
    ) {
        this.file_modal = document.getElementById('updlg-modal-upload-files');

        this.nav_btn_click_events();
        this.submit_event(submit_link_endpoint, submit_input_endpoint, text_on_progress_done);
        this.events();
    }

    nav_btn_click_events() {
        const file_modal = this.file_modal;

        const btns_nav_upload_files = file_modal.querySelectorAll('button.updlg-nav-upload-files')
        const panes_upload_files = file_modal.querySelectorAll('.updlg-pane-upload-files-modal');
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

    submit_event(submit_link_endpoint, submit_input_endpoint, text_on_progress_done) {
        const file_modal = this.file_modal;
        function get_upload_method() {
            const btns = file_modal.querySelectorAll('button.updlg-nav-upload-files')
            for (const btn of btns) {
                if (btn.classList.contains('active')) {
                    return btn.dataset.method;
                }
            }
        }

        const upload_files_submit = this.file_modal.querySelector('#updlg-upload-files-modal-submit')

        upload_files_submit.addEventListener('click', () => {
            const upload_method = get_upload_method();
            if (upload_method === 'link') {
                this.upload_url(submit_link_endpoint);
            }
            else if (upload_method === 'input') {
                this.upload_file(submit_input_endpoint, text_on_progress_done);
            } else {
                console.log(`upload method ${upload_method} is not implemented!`);
            }
        });
    }

    events() {
        const file_modal = this.file_modal;
        const check_force_filename = file_modal.querySelector('#updlg-check-force-filename');
        const force_filename_inp = file_modal.querySelector('#updlg-force-filename');

        check_force_filename.addEventListener('change', () => {
            force_filename_inp.hidden = !check_force_filename.checked;
        });
    }


    reset_modal_fields() {
        const file_modal = document.getElementById('updlg-modal-upload-files');
        gl_open_on_click_el.innerHTML = gl_open_on_click_el_default_html;
        file_modal.querySelector('#updlg-upload-files-input').value = '';
        file_modal.querySelector('#updlg-upload-files-modal-submit').disabled = false;
        file_modal.querySelector('#updlg-nav-upload-files-tab-link').disabled = false;
        file_modal.querySelector('#updlg-upload-files-link').disabled = false;
        file_modal.querySelector('#updlg-upload-files-input').disabled = false;
        file_modal.querySelector('#updlg-file-upload-progress-bar').setAttribute('aria-valuenow', '0');
        file_modal.querySelector('#updlg-file-upload-progress').classList.add('d-none');
        file_modal.querySelector('#updlg-loaded_n_total').innerHTML = "";
        file_modal.querySelector('#updlg-upload-files-status').innerHTML = "";
        file_modal.querySelector('#updlg-upload-files-link').value = "";
        file_modal.querySelector('#updlg-upload-files-100-spinner').hidden = true;
        file_modal.querySelector('#updlg-nav-upload-files-tab-input').disabled = false;
        makeModalBackdropResponsive();
        file_modal.querySelector('#updlg-upload-files-modal-close').disabled = false;
        file_modal.querySelector('#updlg-check-force-filename').disabled = false;
        file_modal.querySelector('#updlg-check-force-filename').checked = false;
        file_modal.querySelector('#updlg-force-filename').disabled = false;
        file_modal.querySelector('#updlg-force-filename').hidden = true;
        file_modal.querySelector('#updlg-force-filename').value = "";
    }

    hide_modal() {
        bootstrap.Modal.getOrCreateInstance(document.getElementById('updlg-modal-upload-files')).hide();
    }

    show_modal() {
        bootstrap.Modal.getOrCreateInstance(document.getElementById('updlg-modal-upload-files')).show();
    }

    process_now_update_until_finished(upload_method) {
        const file_modal = document.getElementById('updlg-modal-upload-files');
        const process_button = file_modal.querySelector('#updlg-upload-files-modal-submit');
        file_modal.querySelector('#updlg-upload-files-modal-close').disabled = true;

        // file_modal.classList.add('modal-static');
        process_button.disabled = true;
        process_button.dataset.loading = 'true';
        makeModalBackdropStatic();
        if (gl_open_on_click_el.innerHTML === gl_open_on_click_el_default_html) {
            gl_open_on_click_el.innerHTML = `<span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span> Uploading`;
        }
        file_modal.querySelector('#updlg-check-force-filename').disabled = true;
        file_modal.querySelector('#updlg-force-filename').disabled = true;

        if (upload_method === 'link') {
            file_modal.querySelector('#updlg-nav-upload-files-tab-input').disabled = true;
            file_modal.querySelector('#updlg-upload-files-link').disabled = true;
            file_modal.querySelector('#updlg-upload-files-100-spinner').hidden = false;
            file_modal.querySelector('#updlg-upload-files-status').innerHTML = 'Uploading file. Please wait...'
        } else if (upload_method === 'input') {
            file_modal.querySelector('#updlg-nav-upload-files-tab-link').disabled = true;
            file_modal.querySelector('#updlg-upload-files-input').disabled = true;
        }
    }

    upload_file(endpoint, text_on_progress_done) {
        const file_modal = this.file_modal;
        const file_input = file_modal.querySelector('#updlg-upload-files-input');
        const file_upload_progress = file_modal.querySelector('#updlg-file-upload-progress');
        const progress_bar = file_modal.querySelector('#updlg-file-upload-progress-bar');
        const upload_files_status = file_modal.querySelector('#updlg-upload-files-status');

        const process_now_update_until_finished = this.process_now_update_until_finished;
        const hide_modal = this.hide_modal;
        const reset_modal_fields = this.reset_modal_fields;

        function progressHandler(event) {
            process_now_update_until_finished('input');
            file_modal.querySelector('#updlg-loaded_n_total').innerHTML = "Uploaded " + event.loaded + " bytes of " + event.total;
            let percent = (event.loaded / event.total) * 100;
            progress_bar.setAttribute('aria-valuenow', Math.round(percent).toString());
            progress_bar.style.width = Math.round(percent).toString() + "%";
            upload_files_status.innerHTML = Math.round(percent).toString() + "% uploaded... please wait";
            if (Math.round(percent) >= 100) {
                upload_files_status.innerHTML = text_on_progress_done;
                file_modal.querySelector('#updlg-upload-files-100-spinner').hidden = false;
            }
        }

        function completeHandler(event) {
            upload_files_status.innerHTML = event.target.responseText;

            if(event.target.status === 200) {
                setTimeout(() => {
                    reset_modal_fields();
                    hide_modal();
                }, 500);
            } else {
                let error_msg = JSON.parse(event.target.responseText);
                reset_modal_fields();
                upload_files_status.innerHTML = error_msg.message;
            }
        }

        function errorHandler(event) {
            upload_files_status.innerHTML = event.target.responseText.message;
        }

        function abortHandler() {
            upload_files_status.innerHTML = "Upload Aborted";
        }

        if (file_input.files.length === 0) {
            return;
        }
        let formdata = new FormData();
        formdata.append("file", file_input.files[0]);
        file_upload_progress.classList.toggle('d-none');
        // modal_submit.classList.toggle('d-none');
        let ajax = new XMLHttpRequest();
        ajax.upload.addEventListener("progress", progressHandler, false);
        ajax.addEventListener("load", completeHandler, false);
        ajax.addEventListener("error", errorHandler, false);
        ajax.addEventListener("abort", abortHandler, false);
        ajax.open("POST", endpoint);
        ajax.send(formdata);
    }

    upload_url(endpoint) {
        const file_modal = this.file_modal;

        const check_force_filename = file_modal.querySelector('#updlg-check-force-filename');
        const force_filename_inp = file_modal.querySelector('#updlg-force-filename');


        function handle_invalid_url() {
            const error = new Error('Invalid URL');
            file_modal.querySelector('#status-url').innerHTML = error.message;
        }

        const file_input = file_modal.querySelector('#updlg-upload-files-link');
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

        if (check_force_filename.checked && force_filename_inp.value !== '') {
            formData['filename'] = force_filename_inp.value;
        }

        this.process_now_update_until_finished('link')
        fetch(endpoint, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(formData)
        }).then(
            response => {
                if (!response.ok) {
                    return response.json()
                        .then(error => {
                            throw new Error(error.message);
                        });
                }
                this.reset_modal_fields();
                return response.json();
            }).then(
                data => {
                    this.reset_modal_fields();
                    this.hide_modal();
                    }).catch(
                        error => {
                            this.reset_modal_fields();
                            file_modal.querySelector('#updlg-upload-files-status').innerHTML = error.message;
                        });
    }
}
