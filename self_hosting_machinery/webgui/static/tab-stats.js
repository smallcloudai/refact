import {init as dash_prime_init, switch_away as dash_prime_switch_away} from "./dashboards/dash_prime.js";
// import {init as dash_teams_init, switch_away as dash_teams_switch_away} from "./dashboards/dash_teams.js";
import {init as dash_users_init, switch_away as dash_users_switch_away} from "./dashboards/dash_users.js";


function render_dashboard(el) {
    el.classList.add('active');
    el.classList.add('main-active');
    let dash_div = document.querySelector(`#${el.dataset.toggle}`);
    dash_div.removeAttribute('hidden');
    switch (el.dataset.toggle) {
        case 'dash-prime':
            dash_prime_init(dash_div);
            break;
        // case 'dash-teams':
        // 	dash_teams_init(dash_div);
        // 	break;
        case 'dash-users':
            dash_users_init(dash_div);
            break;
        case 'dash-models':
            break;
        default:
            console.error(`Render: Unknown dashboard: ${name}`);
    }
}

function switch_away_dashboard() {
    document.querySelectorAll('.dash-nav-btn').forEach((el) => {
        if (el.classList.contains('active')) {
            el.classList.remove('active');
            el.classList.remove('main-active');
            let dash_div = document.querySelector(`#${el.dataset.toggle}`);
            dash_div.setAttribute('hidden', '');
            switch (el.dataset.toggle) {
                case 'dash-prime':
                    dash_prime_switch_away(dash_div);
                    break;
                // case 'dash-teams':
                // 	dash_teams_switch_away(dash_div);
                // 	break;
                case 'dash-users':
                    dash_users_switch_away(dash_div);
                    break;
                case 'dash-models':
                    break;
                default:
                    console.error(`Switch Away: Unknown dashboard: ${el.dataset.toggle}`);
            }
        }
    });
}

export async function init() {
    let req = await fetch('/tab-stats.html');
    document.querySelector('#stats').innerHTML = await req.text();

    document.querySelectorAll('.dash-nav-btn').forEach((el) => {
        el.addEventListener('click', (event) => {
            switch_away_dashboard();
            render_dashboard(event.target);
        });
    });
    document.querySelector('.dash-logo').addEventListener('click', (event) => {
        switch_away_dashboard();
        render_dashboard(document.querySelector('.dash-default-active'));
    });
}

export function tab_switched_here() {
    let dash_set = false;
    document.querySelectorAll('.dash-nav-btn').forEach((el) => {
        if (el.classList.contains('active')) {
            render_dashboard(el);
            dash_set = true;
        }
    });
    if (!dash_set) {
        render_dashboard(document.querySelector('.dash-default-active'));
    }
}


export function tab_switched_away() {
    switch_away_dashboard();
}


export function tab_update_each_couple_of_seconds() {
}
