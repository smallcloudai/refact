
export function get_spinner() {
    const spinner = document.createElement('div');
    const spinner_span = document.createElement('span');
    spinner.className = 'spinner-border';
    spinner.role ='status';
    spinner_span.className ='sr-only';
    spinner.style.scale = '0.5';
    spinner.appendChild(spinner_span);
    return spinner;
}
