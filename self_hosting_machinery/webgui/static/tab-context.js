
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


export async function init() {
    let req = await fetch('/tab-context.html');
    document.querySelector('#context').innerHTML = await req.text();

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