export function init() {
}

const log_container = document.getElementById("server-log-log-container");

function start_log_stream() {
    log_container.textContent = '';
    const streamTextFile = async () => {
        const decoder = new TextDecoder();
        const response = await fetch("/tab-server-log-plain/latest?stream=1");
        const reader = response.body.getReader();

        const processResult = ({ done, value }) => {
            if (done) {
                console.log('Streaming complete');
                return;
            }

            const chunk = decoder.decode(value);
            const isAtBottom = log_container.scrollTop >= (log_container.scrollHeight - log_container.offsetHeight);
            log_container.textContent += chunk;
            let log = log_container.textContent.split('\n')
            log_container.textContent = log.slice(-1000).join("\n")

            if (isAtBottom) {
                log_container.scrollTop = log_container.scrollHeight;
            }
            return reader.read().then(processResult);
        };

        return reader.read().then(processResult);
    }
    streamTextFile()
    .catch(error => {
        console.log('Error:', error);
    });
}

function get_daily_logs() {
    fetch("/tab-server-log-get")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        render_daily_logs(data);
    });
}

function render_daily_logs(data) {
    const log_container = document.querySelector('.daily-logs');
    log_container.innerHTML = '';
    log_container.innerHTML = '<h5>Logs by date</h5>';
    const log = data.all_logs.map((log) => {
        return `<div><a target="_blank" class="link-secondary link-offset-2 link-underline-opacity-25 link-underline-opacity-100-hover" href="/tab-server-log-plain/${log}" class="log-item">${log}</a></div>`;
    }).join("\n");
    log_container.innerHTML += log;
    const last_logs_button = document.querySelector('.latest-log');
    if(data.latest_log && data.latest_log != '') {
        last_logs_button.classList.remove('d-none');
        last_logs_button.href = `/tab-server-log-plain/${data.latest_log}`;
    } else {
        last_logs_button.classList.add('d-none');
    }
}

let is_started = false;
export function tab_switched_here() {
    if(is_started) {
        return;
    }
    get_daily_logs();
    start_log_stream();
    is_started = true;
}