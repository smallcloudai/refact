export function tab_about_version_get() {
    fetch("/tab-about-version-get")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            const tab_version = document.getElementById("refact-version")
            var version_table_data = `<tr><th>Package</th><th>Version</th><th>Commit Hash</th></tr>`;
            data["version_table"].forEach(function(row) {
                version_table_data += `
                    <tr>
                    <td><label class="refact-item-name">${row[0]}</label></td>
                    <td><label class="refact-item-version">${row[1]}</label></td>
                    <td><label class="refact-item-hash">${row[2]}</label></td>
                    </tr>`;
            });
            tab_version.innerHTML = `
                <div><table class="table table-stripped align-left">
                ${version_table_data}
                </table></div>`;
        });
}


export async function init() {
    let req = await fetch('/tab-about.html');
    document.querySelector('#about').innerHTML = await req.text();
}


export function tab_switched_here() {
    tab_about_version_get();
}


export function tab_switched_away() {
}


export function tab_update_each_couple_of_seconds() {
}
