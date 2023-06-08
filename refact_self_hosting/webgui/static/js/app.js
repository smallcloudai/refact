import * as model_hosting_tab from './tab-model-hosting.js';
import * as upload_tab from './tab-upload.js';
import * as finetune_tab from './tab-finetune.js';
import * as access_control_tab from './tab-access-contol.js';

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

const tabs = document.querySelectorAll('.nav-link[data-bs-toggle="tab"]');
tabs.forEach(tab => {
    tab.addEventListener('shown.bs.tab', () => {
        stop_tab_timer();
        start_tab_timer();
    });

    tab.addEventListener('hidden.bs.tab', () => {
        stop_tab_timer();
    });
});

// document.addEventListener('shown.bs.tab', function (e) {
//     switch (e.target.id) {
//         case "model-tab":
//             model_hosting_tab.tab_switched_here();
//             break;
//         case "upload-tab":
//             upload_tab.tab_switched_here();
//             break;
//         case "finetune-tab":
//             finetune_tab.tab_switched_here();
//             break;
//         case "access-control-tab":
//             break;
//     }
// });

function active_tab_function() {
    const activeTab = document.querySelector('.nav-link.active');
    const tabId = activeTab.getAttribute('href');
    console.log('tabId',tabId);

    switch (tabId) {
        case '#model-tab':
            model_hosting_tab.tab_switched_here();
            break;
        case '#upload-tab':
            upload_tab.tab_switched_here();
            break;
        case '#finetune-tab':
            finetune_tab.tab_switched_here();
            break;
        case "#access-control-tab":
            break;
    }
}
let refresh_interval;
function start_tab_timer() {
    console.log('timer started');
    refresh_interval = setInterval(active_tab_function, 10000);
}

function stop_tab_timer() {
    console.log('timer stopped');
    clearInterval(refresh_interval);
}
start_tab_timer();