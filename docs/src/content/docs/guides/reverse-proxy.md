---
title: Self-hosted Refact
description: Setting Up nginx Reverse Proxy for Refact
---

## Why Reverse Proxy?

To access a Refact server from anywhere on the internet, the reverse proxy needs to accept encrypted HTTPS requests and forward them via HTTP to the Refact server.

Transport encryption is not the only thing you need to make the connection secure -- you also need to set a good password.


## Self-Signed Certificate

You can generate a self-signed certificate in one command:

```bash
openssl req -x509 -newkey rsa:4096 -nodes -keyout private_key.key -out certificate.crt -days 365
```

Of course it's better to use a real certificate, because clients will be able to verify it, making
man-in-the-middle attacks impossible.
For a self-signed certificate, you will need to set "Allow insecure server connections when using SSL" option
in the IDE plugin settings.


## Setting Up

The next command assumes you have nginx installed on your server.
It's possible to try it your laptop as well (use `brew install nginx` on a macbook).

```bash
/opt/homebrew/opt/nginx/bin/nginx -g "daemon off;" -c ~/my_reverse_proxy.config
```

The `daemon off;` part allows nginx to run the the current console, so you can quickly stop and restart it.

An example of `my_reverse_proxy.config` that's tested to work with Refact, with comments:

```
http {
  upstream refact-default {
    zone refact-default 1m;
    server 127.0.0.1:8008;    # replace with your address
    keepalive 2;
  }

  server {
    listen  0.0.0.0:443  ssl http2;
    server_name myserver;

    ssl_certificate /your/path/certificate.crt;
    ssl_certificate_key /your/path/private_key.key;

    add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;
    add_header X-Frame-Options SAMEORIGIN                                               always;
    add_header X-Content-Type-Options nosniff                                           always;
    add_header X-XSS-Protection "1; mode=block"                                         always;
    add_header Referrer-Policy                      "no-referrer"                       always;
    add_header X-Permitted-Cross-Domain-Policies    "none"                              always;
    add_header X-Robots-Tag                         "noindex, nofollow"                 always;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection $http_connection;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_buffering off;                     # makes chat streaming possible

    location / {
      allow all;
      proxy_pass http://refact-default;      # here "refact-default" refers to the upstream block above
    }

    error_page 307 = @redirect;
    location @redirect {
      return 307 https://$host$request_uri;
    }

    client_max_body_size 1G;

    ssl_protocols TLSv1.2 TLSv1.3;       # The refact-lsp binary does not support TLSv1.3 yet -- waiting for reqwest library to catch up
    ssl_prefer_server_ciphers on;
    ssl_ciphers EECDH+AESGCM:EDH+AESGCM;
    ssl_ecdh_curve secp384r1;
    ssl_session_timeout  10m;
    ssl_session_cache shared:SSL:10m;
    ssl_session_tickets off;
    ssl_stapling on;
    ssl_stapling_verify on;
    resolver_timeout 5s;
  }
}

events {
  worker_connections 1024;
  multi_accept on;
}
```

## Debugging

Refact plugins use `refact-lsp` to communicate with the server. It starts together with the IDE as a subprocess. Here is a command to find it when it's running:

```
ps aux | grep refact-lsp
```

You can quickly run `refact-lsp` in the terminal to see if your proxy works. If it doesn't, you'll see the error immediately:

```
PATH_TO_BINARY/refact-lsp --address-url https://127.0.0.1:443/ --api-key XXX --http-port 8001 --logs-stderr
```

where PATH_TO_BINARY you can copy-paste from the previous command. The line you are looking for is:

```
2023-12-01T19:42:54.208374Z  INFO refact_lsp::caps:264: reading caps from https://127.0.0.1:443/coding_assistant_caps.json
```

If that works, and it really has "https" in the address, then the proxy is working correctly!

If it doesn't, you can look at the error message, and also you can try to fetch the same file using curl:

```bash
curl https://127.0.0.1:443/coding_assistant_caps.json
```

And compare the results. Please report any problems following this page!
