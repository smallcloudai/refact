document.getElementById("login-form").addEventListener('submit', (event) => {
    event.preventDefault();

    const token = document.getElementById("token").value;

    fetch('/login', {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            "token": token
        })
    }).then(response => response.json()).then(data => {
        if (data.session_key) {
            document.cookie = `session_key=${data.session_key}; path=/`;
            window.location.href = "/";
        } else {
            const login_failed = document.getElementById("login-failed");
            login_failed.classList.remove("d-none");
        }
    }).catch(error => console.log(error));
});

["token"].forEach((input_name) => {
    document.getElementById(input_name).addEventListener('focus', (event) => {
        event.preventDefault();
        const login_failed = document.getElementById("login-failed");
        login_failed.classList.add("d-none");
    });
});
