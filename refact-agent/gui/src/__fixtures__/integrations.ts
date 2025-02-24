import type { Integration } from "../services/refact";

export const INTEGRATIONS_RESPONSE: Integration = {
  project_path: "",
  integr_name: "postgres",
  integr_config_path: "/Users/user/.config/refact/integrations.d/postgres.yaml",
  integr_schema: {
    fields: {
      host: {
        f_type: "string",
        f_desc:
          "Connect to this host, for example 127.0.0.1 or docker container name.",
        f_placeholder: "marketing_db_container",
      },
      port: {
        f_type: "int",
        f_desc: "Which port to use.",
        f_default: "5432",
      },
      user: {
        f_type: "string",
        f_placeholder: "john_doe",
      },
      password: {
        f_type: "string",
        f_default: "$POSTGRES_PASSWORD",
        smartlinks: [
          {
            sl_label: "Open passwords.yaml",
            sl_goto: "EDITOR:passwords.yaml",
          },
        ],
      },
      database: {
        f_type: "string",
        f_placeholder: "marketing_db",
      },
    },
    confirmation: {
      ask_user_default: [],
      deny_default: [],
    },
    available: {
      on_your_laptop_possible: true,
      when_isolated_possible: true,
    },
    smartlinks: [
      {
        sl_label: "Test",
        sl_chat: [
          {
            role: "user",
            content:
              "🔧 Use postgres database to briefly list the tables available, express satisfaction and relief if it works, and change nothing.\nIf it doesn't work, go through the usual plan in the system prompt.\nThe current config file is @file %CURRENT_CONFIG%\n",
          },
        ],
      },
    ],
    docker: {
      filter_image: "postgres",
      filter_label: "",
      new_container_default: {
        image: "postgres:13",
        environment: {
          POSTGRES_DB: "marketing_db",
          POSTGRES_USER: "john_doe",
          POSTGRES_PASSWORD: "$POSTGRES_PASSWORD",
        },
      },
      smartlinks: [
        {
          sl_label: "Add Database Container",
          sl_chat: [
            {
              role: "user",
              content:
                '🔧 Your job is to create a new section under "docker" that will define a new postgres container, inside the current config file %CURRENT_CONFIG%. Follow the system prompt.\n',
            },
          ],
        },
      ],
    },
  },
  integr_values: {
    psql_binary_path: "",
    host: "",
    port: 0,
    user: "",
    password: "",
    database: "",
  },
  error_log: [],
};
