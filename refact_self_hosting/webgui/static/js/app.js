import * as model_hosting_tab from './tab-model-hosting.js';
import * as upload_tab from './tab-upload.js';
import * as finetune_tab from './tab-finetune.js';
import * as access_control_tab from './tab-access-contol.js';
import * as server_log_tab from './tab-server-logs.js';
import * as ssh_settings_tab from './tab-ssh-settings.js';
import * as apikey_settings_tab from './tab-api-key-settings.js';

let comming_soon;

function display_comming_soon() {
    comming_soon = document.querySelectorAll(".temp-disabled");
    comming_soon_render();
    window.addEventListener("resize", function () {
        comming_soon_resize();
    });

    document.addEventListener('shown.bs.tab', function (e) {
        comming_soon_resize();
    });
}
function comming_soon_render() {
    comming_soon.forEach(function (element) {
        const info = element.parentNode.insertBefore(document.createElement("div"), element.nextSibling);
        info.classList.add("temp-info");
        info.innerHTML = "Coming soon";
        info.style.marginLeft = ((element.getBoundingClientRect().width / 2) - (info.getBoundingClientRect().width / 2)) + "px";
        info.style.marginTop = ((element.getBoundingClientRect().height / 2) * -1 - (info.getBoundingClientRect().height / 2)) + "px";
    });
}
function comming_soon_resize() {
    comming_soon.forEach(function (element) {
        const info = element.nextSibling;
        info.style.marginLeft = ((element.getBoundingClientRect().width / 2) - (info.getBoundingClientRect().width / 2)) + "px";
        info.style.marginTop = ((element.getBoundingClientRect().height / 2) * -1 - (info.getBoundingClientRect().height / 2)) + "px";
    });
}
display_comming_soon();
model_hosting_tab.init();
upload_tab.init();
finetune_tab.init();
access_control_tab.init();
server_log_tab.init();
ssh_settings_tab.init();
apikey_settings_tab.init();

const tabs = document.querySelectorAll('.nav-link[data-bs-toggle="tab"]');
tabs.forEach(tab => {
    tab.addEventListener('shown.bs.tab', () => {
        start_tab_timer();
    });
});

function active_tab_function() {
    const active_tab = document.querySelector('.nav-link.active');
    switch (active_tab.id) {
        case 'model-tab':
            model_hosting_tab.tab_switched_here();
            break;
        case 'upload-tab':
            upload_tab.tab_switched_here();
            break;
        case 'finetune-tab':
            finetune_tab.tab_switched_here();
            break;
        case 'logs-tab':
            server_log_tab.tab_switched_here();
            break;
        case 'settings-tab':
            ssh_settings_tab.tab_switched_here();
            break;
        case 'api-keys-tab':
            apikey_settings_tab.tab_switched_here();
            break;
        case "access-control-tab":
            break;
    }
}

let refresh_interval = null;

function start_tab_timer() {
    active_tab_function();
    if (refresh_interval) {
        clearInterval(refresh_interval);
    }
    refresh_interval = setInterval(active_tab_function, 1000);
}

start_tab_timer();
