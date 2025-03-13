---
title: Keycloak Integration Guide
description: Step-by-step instructions for integrating Keycloak with Refact.ai Enterprise for access management.
---

Keycloak provides a secure solution for Identity and Access Management (IAM). This 
guide will walk you through the steps to integrate Keycloak with Refact.ai Enterprise.

## Setting Up Keycloak

### Accessing the Keycloak Console

1. Begin by navigating to the Keycloak console in your browser.

### Choosing a Realm

2. **Realm Selection**:
   - **For a New Realm**: Click `Create Realm`. Provide a name for your new realm and proceed.
   - **For an Existing Realm**: Select an existing realm from the dropdown menu if applicable.
   ![Keycloak Console](../../../../assets/keycloak_console.png)

### Creating and Configuring the Client

3. Navigate to the `Clients` tab.
   ![Clients Tab](../../../../assets/clients.png)

4. Click `Create client` and enter the following details:
   - **Client ID**: (e.g., `refact_client`)
   - **Name**: (e.g., `Refact Client`)
     ![Client Creation Form](../../../../assets/create_client.png)

5. Adjust the **Capability config** to:
   - Enable `Client Authentication`
   - Set `Authorization` to OFF
   - For `Authentication Flow`, select only `Direct Access Grants`, `Service Accounts Roles`, and `Standard flow` and deselect other options.
     ![Capabilities Config](../../../../assets/capabilities_config.png)

6. Configure the **Access Settings** as follows:
   - **Valid Redirect URIs**: The URL of your Refact.ai Enterprise inference. For example, `https://enterprise.inference-server.local/*` (replace `enterprise.inference-server.local` with your Refact.ai Enterprise URL and make sure to include the trailing slash and an asterisk at the end)
   - **Web Origins**: The URL of your Refact.ai Enterprise inference. For example, `https://enterprise.inference-server.local/`
   ![Access Settings](../../../../assets/access_settings.png)

7. Leave both `Root URL` and `Home URL` fields empty in the `Login Settings` tab.
     ![Login Settings](../../../../assets/login_settings.png)

## Adding a Service Role to the Client

1. In your newly created client, add a service role.
   ![Refact Client](../../../../assets/refact_client.png)

2. Click `Assign role` and modify `Filter by realm roles` to `Filter to clients`. Then, in the search field, input `view-users`.
   ![Role Assignment](../../../../assets/assign_role_first.png)
   ![Adding View-users Role](../../../../assets/view_users.png)

3. Go to the `Credentials` tab, locate, and save the `Client Secret` value.
   ![Client Secret](../../../../assets/client_secret.png)

### Configuration Summary

Ensure your settings are as follows for successful integration:

```
client_id = refact_client
client_secret = ***** (Your generated client secret)
realm = your_realm_name
url = https://keycloak.refact.ai/
```


## Integrating Keycloak with Refact.ai Enterprise


### Regular User Flow

1. Navigate to your Refact.ai Enterprise instance. Press `Continue to Keycloak`. 
   ![Refact Enterprise Login](../../../../assets/login_keycloak.png)
You will be redirected to the Keycloak, enter your credentials and click `Sign in`.

2. You will be redirected to your Refact.ai Enterprise instance. You will see your user profile information:
   - Account Login
   - Plugin API Key
   - Your team

  ![Refact Enterprise User](../../../../assets/user_info.png)

### Admin User Flow

1. Navigate to your Refact.ai Enterprise instance. Press `Administrator login`. 
   ![Refact Enterprise Login](../../../../assets/login_keycloak.png)

2. Fill in your Refact.ai admin token.
   ![Admin Console](../../../../assets/admin_console.png)

3. Press the `Auth` tab in the `settings` dropdown.
   ![Refact Integrations](../../../../assets/refact_integrations.png)

4. Input the previously configured Keycloak settings. Confirm by clicking `Save Settings`.
  ![Keycloak Settings](../../../../assets/keycloak_settings.png)

