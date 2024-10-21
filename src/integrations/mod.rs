pub mod integr_github;
pub mod integr_pdb;
pub mod integr_chrome;
pub mod sessions;
pub mod process_io_utils;

pub const INTEGRATIONS_DEFAULT_YAML: &str = r#"# This file is used to configure integrations in Refact Agent.
# If there is a syntax error in this file, no integrations will work.
#
# Here you can set up which commands require confirmation or must be denied. If both apply, the command is denied.
# Rules use glob patterns for wildcard matching (https://en.wikipedia.org/wiki/Glob_(programming))
#

commands_need_confirmation:
  - "gh * delete*"
commands_deny:
  - "gh auth token*"


# --- GitHub integration ---
#github:
#  gh_binary_path: "/opt/homebrew/bin/gh"  # Uncomment to set a custom path for the gh binary, defaults to "gh"
#  GH_TOKEN: "GH_xxx"                      # To get a token, check out https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens


# --- Pdb integration ---
#pdb:
#  python_path: "/opt/homebrew/bin/python3"  # Uncomment to set a custom python path, defaults to "python3"


# --- Chrome integration ---
chrome:
#  chrome_path: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"  # Uncomment to set a custom path for the chrome binary, defaults to "
  window_size: [1024, 768]  # Size of the window, defaults to [1024, 768]
  idle_browser_timeout: 60  # Timeout in seconds for idle browsers, defaults to 60 seconds

"#;
