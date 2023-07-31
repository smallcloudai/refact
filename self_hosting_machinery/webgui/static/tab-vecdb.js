
function is_vecdb_enabled() {
    let vecdb_tab = document.getElementById('vecdb-tab');
    let data_active = vecdb_tab.getAttribute('data-active');
    return data_active === 'true';
}

function fetch_and_set_health() {
    fetch("/tab-vecdb-health")
        .then(function(response) {
            if (response.ok) {
                return response.json();
            }
            return {'display_text': 'error'}
        })
        .then(function(data) {
            document.querySelector('#vecdb-health').innerHTML = data['display_text'];
        });
}


function fetch_and_set_files_loaded_cnt() {
    fetch("/tab-vecdb-files-stats")
        .then(function(response) {
            if (response.ok) {
                return response.json();
            }
            return {'files_cnt': 'error', 'chunks_cnt': 'error'}
        })
        .then(function(data) {
            document.querySelector('#vecdb-files-loaded-cnt').innerHTML = data['files_cnt'];
            document.querySelector('#vecdb-chunks-loaded-cnt').innerHTML = data['chunks_cnt'];
        });
}


async function fetch_and_set_vecdb_url() {
    await fetch('/tab-vecdb-get-url')
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            document.querySelector('#vecdb-url').value = data['url'];
        });
}

function on_url_save_btn_click(event) {
    function sleep (time) {
      return new Promise((resolve) => setTimeout(resolve, time));
    }

    function check_and_set_url() {
        let url_input_value = document.querySelector('#vecdb-url').value;
        fetch(
            "/tab-vecdb-save-url",
            {
                method: "POST",
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({'url': url_input_value})
            }).then(response => {
            if (!response.ok) {
                return response.json()
                    .then(error => {
                        throw new Error(error.message);
                    });
            }
            return response.json();
        });
    }
    check_and_set_url();
    sleep(100).then(() => {
        fetch_and_set_health();
        fetch_and_set_files_loaded_cnt();
    })
}


async function on_delete_all_btn_click(event) {

}

async function on_status_refresh_btn_click(event) {
    fetch_and_set_health();
    fetch_and_set_files_loaded_cnt();
}

async function on_test_request_btn_click(event) {
    let test_request_input = document.getElementById('vecdb-test-request-input');
    let test_request_container = document.getElementById('vecdb-test-request-container');
    test_request_container.innerHTML = '';

    let query_text = test_request_input.value;
    fetch(
        '/tab-vecdb-find',
        {
            method: "POST",
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({'query': query_text})
        }
    ).then(response => {
        return response.json();
    }).then(data => {
        let error = data['error'];
        if (error && error !== '') {
            test_request_container.innerHTML = error;
        } else {
            test_request_container.innerHTML = data['text'];
        }
    })
}


export async function init() {
    if (!is_vecdb_enabled()) {
        return;
    }
    let req = await fetch('/tab-vecdb.html');
    document.querySelector('#vecdb').innerHTML = await req.text();

    let url_save_btn = document.getElementById('vecdb-url-save-btn');
    url_save_btn.addEventListener('click', on_url_save_btn_click);

    let status_refresh_btn = document.getElementById('vecdb-status-refresh-btn');
    status_refresh_btn.addEventListener('click', on_status_refresh_btn_click);

    let delete_all_modal = document.getElementById('vecdb-delete-all-modal');
    let delete_all_btn = document.getElementById('vecdb-delete-all-btn');
    delete_all_btn.addEventListener('click', () => {
        let delete_modal_instance = bootstrap.Modal.getOrCreateInstance(delete_all_modal);
        delete_modal_instance.show();
    });

    let test_request_btn = document.getElementById('vecdb-test-request-btn');
    test_request_btn.addEventListener('click', on_test_request_btn_click);

    await fetch_and_set_vecdb_url();
    fetch_and_set_health();
    fetch_and_set_files_loaded_cnt();
}


export async function tab_switched_here() {
    await init();
}


export async function tab_switched_away() {

}


export async function tab_update_each_couple_of_seconds() {
    if (is_vecdb_enabled()) {
        fetch_and_set_health();
    }
}