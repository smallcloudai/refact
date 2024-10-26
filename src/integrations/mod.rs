pub mod integr_github;
pub mod integr_gitlab;
pub mod integr_pdb;
pub mod integr_chrome;
pub mod sessions;
pub mod process_io_utils;
pub mod integr_postgres;

pub const INTEGRATIONS_DEFAULT_YAML: &str = r#"# This file is used to configure integrations in Refact Agent.
# If there is a syntax error in this file, no integrations will work.
#
# Here you can set up which commands require confirmation or must be denied. If both apply, the command is denied.
# Rules use glob patterns for wildcard matching (https://en.wikipedia.org/wiki/Glob_(programming))
#

commands_need_confirmation:
  - "gh * delete*"
  - "glab * delete*"
  - "psql*[!SELECT]*"
commands_deny:
  - "gh auth token*"
  - "glab auth token*"


# --- GitHub integration ---
#github:
#   gh_binary_path: "/opt/homebrew/bin/gh"  # Uncomment to set a custom path for the gh binary, defaults to "gh"
#   GH_TOKEN: "GH_xxx"                      # To get a token, check out https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens


# --- GitLab integration ---
#gitlab:
#   glab_binary_path: "/opt/homebrew/bin/glab"  # Uncomment to set a custom path for the glab binary, defaults to "glab"
#   GITLAB_TOKEN: "GL_xxx"                      # To get a token, check out https://docs.gitlab.com/ee/user/profile/personal_access_tokens


# --- Pdb integration ---
#pdb:
#  python_path: "/opt/homebrew/bin/python3"  # Uncomment to set a custom python path, defaults to "python3"


# --- Chrome integration ---
chrome:
#  chrome_path: "/path/to/chrome"  # can be path to your binary or opened debug_ws_url (see --remote-debugging-port)
  window_size: [1024, 768]   # Size of the window, defaults to [1024, 768]
  idle_browser_timeout: 600  # Timeout in seconds for idle browsers, defaults to 600 seconds

# --- Postgres integration ---
#postgres:
#  psql_binary_path: "/path/to/psql"  # Uncomment to set a custom path for the psql binary, defaults to "psql"
#  connection_string: "postgresql://username:password@localhost/dbname"  # To get a connection string, check out https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING

"#;
