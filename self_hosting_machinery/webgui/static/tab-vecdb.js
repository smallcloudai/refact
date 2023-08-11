
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


async function on_upload_files_btn_click(event) {
    fetch('/tab-vecdb-upload-files').then(response => {
        if (response.ok) {
            return response.json();
        } else {
            return {'status': 'unknown error'}
        }
    }).then(data => {
        document.querySelector('#vecdb-upload-files-status').innerHTML = data['status'];
    });
}

export async function init() {
    let req = await fetch('/tab-vecdb.html');
    document.querySelector('#vecdb').innerHTML = await req.text();

    let upload_files_btn = document.getElementById('vecdb-upload-files-btn');
    upload_files_btn.addEventListener('click', on_upload_files_btn_click);

    let status_refresh_btn = document.getElementById('vecdb-status-refresh-btn');
    status_refresh_btn.addEventListener('click', on_status_refresh_btn_click);

    fetch_and_set_health();
    fetch_and_set_files_loaded_cnt();
}


export async function tab_switched_here() {
    fetch_and_set_health();
    fetch_and_set_files_loaded_cnt();
}


export async function tab_switched_away() {

}


export async function tab_update_each_couple_of_seconds() {
    fetch_and_set_health();
}