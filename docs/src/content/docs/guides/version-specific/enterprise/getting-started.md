---
title: Enterprise Refact Edition - Getting Started
description: What Enterprise Refact is and how it works.
---

Enterprise self-hosted version of Refact allows you to deploy various code models for a local AI code assistant inside your IDE. It also allows you to create a fine-tuned model on your company's codebase. 
The enterprise plan is designed for teams who want to have full control over their Refact experience and access to all features.

## Prerequisites

:::note
This and the following step are required to deploy the Refact server in a local environment. If you are using services like AWS or Runpod, read one of the following guides:
- [Runpod Guide](https://docs.refact.ai/guides/deployment/runpod/)
- [AWS Guide](https://docs.refact.ai/guides/deployment/aws/getting-started)
:::

- Docker with GPU support. Follow the link to [install Docker](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html#docker). On Windows you need to install WSL 2 first. Follow the guide to [install WSL 2](https://docs.docker.com/desktop/install/windows-install).

## Pull Docker Image

Run the following in your terminal:
```
docker pull smallcloud/refact_self_hosting_enterprise:latest
wget https://docs.refact.ai/docker-compose.yml
```
:::note
If you have used the enterprise with a `beta` tag before, please ensure you use the `latest` tag from now.
:::
[Download](https://docs.refact.ai/docker-compose.yml) the `docker-compose.yml` file and run the docker `compose up` command in your terminal.

## Generating a Random Admin Password (Optional)

The Refact server is designed to be safe to expose to the internet. To do it correctly, make sure you don't skip these two steps:
1. Generate a random password using the `openssl` utility:
    ```
    openssl rand -base64 15
    ```
    
    Add the result to `docker-compose.yml` file, the `ENTERPRISE_ADMIN_TOKEN` section.

2. Set up a Reverse Proxy that will handle incoming HTTPS requests and forward them to your Refact server running on HTTP (port 8008). The specific setup depends on what your organization uses.

Refact IDE plugins can connect to the Refact server using either HTTP or HTTPS protocols.

The unencrypted HTTP is fine when using a local network or VPN. But the plugins will require a valid SSL/TLS certificate for HTTPS connection. Ask the administrator in your company about setting up a reverse proxy and obtaining a valid certificate.

Server Web UI requires an admin password to log in. If you forgot the password, you can delete the container and run it again. The `docker-compose.yml` defines persistent volumes to store all the important data, they will survive container restart, kill/run cycle or upgrade.

## User Management

To find out how to manage users, refer to the [User Creation](https://docs.refact.ai/guides/version-specific/enterprise/users/#create-a-user) section.