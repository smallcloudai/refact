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

let is_started = false;

export function tab_switched_here() {
    if (!is_started) {
        start_log_stream();
        is_started = true;
    }
}