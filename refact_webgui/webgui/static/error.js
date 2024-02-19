export const general_error = error => {
    // if(show_general_error) { return; }
    const error_toast = document.querySelector('.global-error-toast');
    const error_toast_content = error_toast.querySelector('.toast-body');
    error_toast_content.innerHTML = error;
    if(error.details && error.details.length > 0) {
        error_toast_content.innerHTML = error.details;
    }
    const error_toast_box = bootstrap.Toast.getOrCreateInstance(error_toast);
    error_toast_box.show();
    // show_general_error = true;

    const toast_close = document.querySelector('.global-error-toast-close');
    toast_close.addEventListener('click', function() {
        error_toast_content.innerHTML = '';
        // show_general_error  = false;
    });
}