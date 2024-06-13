let stored_theme = localStorage.getItem('theme') || (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
if (stored_theme) {
    document.documentElement.setAttribute('data-bs-theme', stored_theme)
}

document.getElementById("login-form").addEventListener('submit', (event) => {
    event.preventDefault();
    const token = document.getElementById("token").value;
    fetch('/admin', {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            "token": token
        })
    })
    .then(response => response.json())
    .then(data => {
        if (data.session_key) {
            document.cookie = `session_key=${data.session_key}; path=/`;
            window.location.href = "/";
        } else {
            const login_failed = document.querySelector('.token-error');
            login_failed.classList.remove("d-none");
        }
    })
    .catch(error => console.log(error));
});

["token"].forEach((input_name) => {
    document.getElementById(input_name).addEventListener('focus', (event) => {
        event.preventDefault();
        const login_failed = document.querySelector('.token-error');
        login_failed.classList.add("d-none");
    });
});

const site_mode = document.querySelector('.login-mode');


 site_mode.addEventListener('click', () => {
     var current_theme = document.documentElement.getAttribute("data-bs-theme");
     var target_theme = "light";

     if (current_theme === "light") {
         target_theme = "dark";
     }

     document.documentElement.setAttribute('data-bs-theme', target_theme)
     localStorage.setItem('theme', target_theme);
 });