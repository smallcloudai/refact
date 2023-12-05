let plots_data = {};

export async function fetch_plots_data(dash_name) {
    if (plots_data[dash_name] === undefined) {
        try {
            console.log(`Fetching dashboard data: ${dash_name}`);
            const response = await fetch(`/stats/${dash_name}`);

            if (!response.ok) {
                throw new Error(`Failed to fetch dashboard data: ${dash_name}; ${response.status} ${response.statusText}`);
            }
            plots_data[dash_name] = await response.json();
        } catch (error) {
            console.error(error);
            throw error;
        }
    }
    return plots_data[dash_name];
}

export async function fetch_teams_dashboard_data(data) {
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
            throw new Error(`Failed to fetch dashboard data: dash-teams; ${response.status} ${response.statusText}`);
        }
        return await response.json();
    } catch (error) {
        console.error(error);
        throw error;
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
