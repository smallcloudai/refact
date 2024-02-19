import {fill_table_with_data} from "./utils.js";


export function render_plot_with_buttons(wrap_id, plot_data, create_plot_func, plot_class = "dash-plot-md",) {
    const btns_data = plot_data['btns_data'];

    let btn_group = document.createElement('div');
    btn_group.id = `${wrap_id}-btn-group`;
    document.querySelector(`#${wrap_id}`).appendChild(btn_group);

    let plot_t = document.createElement('div');
    plot_t.id = `${wrap_id}-plot`;
    if (plot_class === "dash-plot-md") {
        plot_t.style.width = "1000px";
        plot_t.style.height = "400px";
    } else if (plot_class === "dash-plot-sm") {
        plot_t.style.width = "900px";
        plot_t.style.height = "400px";
    }
    document.querySelector(`#${wrap_id}`).appendChild(plot_t);
    let plot = document.querySelector(`#${wrap_id}-plot`);

    let insert_in_el = document.querySelector(`#${wrap_id}-btn-group`);
    insert_in_el.style.textAlign = "right";
    insert_in_el.style.width = plot_t.style.width;

    for (const [_, btn_text] of Object.entries(btns_data['btns_text'])) {
        let btn_id = `${wrap_id}-btn-${btn_text}`;

        let btn = document.createElement('button');
        btn.type = "button";
        btn.className = "btn btn-outline-secondary btn-sm dash-plot-btn";
        btn.id = btn_id;
        btn.innerText = btn_text;
        btn.setAttribute("data-target", btn_text);
        insert_in_el.appendChild(btn);

        if (btn_text === btns_data['default']) {
            btn.classList.add('active');
            create_plot_func(plot_data[btn_text], plot);
        }
    }

    document.querySelectorAll(`#${insert_in_el.id} button`).forEach((el) => {
        el.addEventListener('click', (event) => {
            document.querySelectorAll(`#${insert_in_el.id} button`).forEach((el0) => {
                el0.classList.remove('active');
            });
            event.target.classList.add('active');
            for (const [_, btn_text] of Object.entries(btns_data['btns_text'])) {
                if (btn_text === event.target.dataset.target) {
                    create_plot_func(plot_data[btn_text], plot);
                }
            }
        });
    });
}

export function barplot_rh(barplot_rh_data, insert_in_el) {
    let option = {
        title: {
            text: barplot_rh_data["title"]
        }, tooltip: {
            trigger: 'axis', axisPointer: {
                type: 'cross', crossStyle: {
                    color: '#999'
                }
            }
        }, toolbox: {}, legend: {
            data: ['Assistant', 'Human', 'A/(A+H)']
        }, xAxis: [{
            type: barplot_rh_data["x_axis_type"], data: barplot_rh_data["x_axis"], axisPointer: {
                type: 'shadow'
            }
        }], yAxis: [{
            type: 'value', name: 'Characters',
        }, {
            type: 'value', name: 'A/(A+H)', axisLabel: {
                formatter: '{value}',
            }
        }], series: [{
            name: 'Assistant', type: 'bar', tooltip: {
                valueFormatter: function (value) {
                    return value + ' characters';
                }
            }, data: barplot_rh_data["data"]["robot"]
        }, {
            name: 'Human', type: 'bar', tooltip: {
                valueFormatter: function (value) {
                    return value + ' characters';
                }
            }, data: barplot_rh_data["data"]["human"]
        }, {
            name: 'A/(A+H)', type: 'line', yAxisIndex: 1, tooltip: {
                valueFormatter: function (value) {
                    return value;
                }
            }, data: barplot_rh_data["data"]["ratio"]
        }], dataZoom: [{}],
    };
    if (barplot_rh_data["date_kind"] === 'daily') {
        option["dataZoom"] = [{
            type: 'slider',
            startValue: barplot_rh_data["x_axis"][barplot_rh_data["x_axis"].length - 30],
            end: 100,
            showDataShadow: true,
            filterMode: 'filter'
        }];
    }
    let my_chart = echarts.init(insert_in_el);
    my_chart.setOption(option);
}


export function barplot_completions(comp_data, insert_in_el) {
    let option = {
        title: {
            text: comp_data["title"]
        }, legend: {
            data: ['Completions']
        }, xAxis: {
            type: comp_data["x_axis_type"], data: comp_data["x_axis"], axisPointer: {
                type: 'shadow'
            }
        }, yAxis: [{
            type: 'value', name: 'Completions',

        }], dataZoom: [{}], tooltip: {
            trigger: 'axis', axisPointer: {
                type: 'shadow'
            }
        }, series: [{
            data: comp_data["data"]["completions"], type: 'bar', tooltip: {
                valueFormatter: function (value) {
                    return value + ' completions';
                }
            },
        },]
    };
    if (comp_data["date_kind"] === 'daily') {
        option["dataZoom"] = [{
            type: 'slider',
            startValue: comp_data["x_axis"][comp_data["x_axis"].length - 30],
            end: 100,
            showDataShadow: true,
            filterMode: 'filter'
        }];
    }

    let my_chart = echarts.init(insert_in_el);
    my_chart.setOption(option);
}


export function barplot_users(users_data, insert_in_el) {
    let option = {
        title: {
            text: users_data["title"]
        }, legend: {
            data: ['Users']
        }, xAxis: {
            type: users_data["x_axis_type"], data: users_data["x_axis"]
        }, yAxis: [{
            type: 'value', name: 'Users',

        }], dataZoom: [{}], tooltip: {
            trigger: 'axis', axisPointer: {
                type: 'shadow'
            }
        }, series: [{
            data: users_data["data"]["users"], type: 'bar', tooltip: {
                valueFormatter: function (value) {
                    return value + ' users';
                }
            }, itemStyle: {color: '#92CC76'}
        },]
    };
    if (users_data["date_kind"] === 'daily') {
        option["dataZoom"] = [{
            type: 'slider',
            startValue: users_data["x_axis"][users_data["x_axis"].length - 30],
            end: 100,
            showDataShadow: true,
            filterMode: 'filter'
        }];
    }

    let my_chart = echarts.init(insert_in_el);
    my_chart.setOption(option);
}

export function barplot_completions_users(comp_data_users, insert_in_el) {
    let series = [];
    for (const [user, user_data] of Object.entries(comp_data_users["data"])) {
        let user_series = {
            name: user, type: 'bar', data: user_data, stack: 'default', tooltip: {
                valueFormatter: function (value) {
                    return value + ' completions';
                }
            },
        };
        series.push(user_series);
    }

    let option = {
        title: {
            text: comp_data_users["title"]
        },
        xAxis: {
            type: 'category', data: comp_data_users["x_axis"], axisPointer: {
                type: 'shadow'
            }
        }, yAxis: [{
            type: 'value', name: 'Completions',

        }], dataZoom: [{}], tooltip: {
            trigger: 'axis', axisPointer: {
                type: 'shadow'
            }
        }, series: series
    };
    if (comp_data_users["date_kind"] === 'daily') {
        option["dataZoom"] = [{
            type: 'slider',
            startValue: comp_data_users["x_axis"][comp_data_users["x_axis"].length - 30],
            end: 100,
            showDataShadow: true,
            filterMode: 'filter'
        }];
    }
    let my_chart = echarts.init(insert_in_el);
    my_chart.off('click');

    my_chart.setOption(option);
    try {
        document.getElementById(`${insert_in_el.id}-table-wrapper`).remove();
    } catch (e) {}
    let table_wrapper = document.createElement('div');
    table_wrapper.id = `${insert_in_el.id}-table-wrapper`;
    insert_in_el.parentElement.appendChild(table_wrapper);
    // table_wrapper.innerText = "Click on a bar to see the table";

    function on_click(params) {
        console.log('comp_data_users', comp_data_users);
        console.log(params);
        if (params.componentType !== 'series' && params.componentSubType !== 'bar' && params.type !== 'click') {
            return;
        }
        let name = params.name;
        let table_data = comp_data_users["table_data"][name];
        let columns = comp_data_users["table_cols"]
        // table_wrapper.innerText = name;

        let table_title = document.createElement('h5');
        table_title.innerText = name;

        let table = document.createElement('table');
        table.className = "table table-striped";
        table.style.width = "100%";
        table.style.height = "auto";
        table.style.marginTop = "10px";
        table_wrapper.innerHTML = "";

        table_wrapper.appendChild(table_title);
        table_wrapper.appendChild(table);


        console.log('table_data', table_data);
        console.log('columns', columns);
        fill_table_with_data(table, {"columns": columns, "data": table_data});
    }

    my_chart.on('click', function (params) {
        on_click(params);
    });
}
