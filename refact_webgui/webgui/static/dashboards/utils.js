import {general_error} from "../error.js";

let plots_data = {};


async function parse_and_display_error(response) {
    try {
        const error = await response.json();
        const error_msg = error['reason'];
        document.querySelector('#dash-error').hidden = false;
        document.querySelector('#dash-error h5').innerText = error_msg;
    }
    catch (error){}
}

export async function fetch_plots_data(dash_name) {
    document.querySelector('#dash-error').hidden = true;
    if (plots_data[dash_name] === undefined) {
        try {
            const response = await fetch(`/stats/${dash_name}`);
            if (!response.ok) {
                await parse_and_display_error(response);
                throw new Error(`Failed to fetch dashboard data: ${dash_name}; ${response.status} ${response.statusText}`);
            }
            plots_data[dash_name] = await response.json();
        } catch (error) {
            console.log('fetch_plots_data', error);
            general_error(error);
        }
    }
    return plots_data[dash_name];
}

export async function fetch_teams_dashboard_data(data) {
    document.querySelector('#dash-error').hidden = true;
    // post request to /dash-teams/generate-dashboard
    // data = {users_selected: [user1, user2, ...]}
    // response = {dashboard_html: html}
    try {
        console.log(`Fetching dashboard data: dash-teams`);
        const response = await fetch(`/stats/dash-teams`, {
            method: 'POST', headers: {
                'Content-Type': 'application/json'
            }, body: JSON.stringify(data),
        });

        if (!response.ok) {
            await parse_and_display_error(response);
            throw new Error(`Failed to fetch dashboard data: dash-teams; ${response.status} ${response.statusText}`);
        }
        return await response.json();
    } catch (error) {
        console.log('fetch_teams_dashboard_data', error);
        general_error(error);
    }
}


export function fill_table_with_data(table, data) {
    let thead = table.createTHead();
    let row = thead.insertRow();
    data.columns.forEach(function (column) {
        let th = document.createElement("th");
        let text = document.createTextNode(column);
        th.appendChild(text);
        row.appendChild(th);
    });

    // Create table body
    let tbody = table.createTBody();
    data.data.forEach(function (rowData) {
        let row = tbody.insertRow();
        rowData.forEach(function (cellData) {
            let cell = row.insertCell();
            let text = document.createTextNode(cellData);
            cell.appendChild(text);
        });
    });
}
