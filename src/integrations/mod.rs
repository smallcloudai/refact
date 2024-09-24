pub mod integr_github;

pub const INTEGRATIONS_DEFAULT_YAML: &str = r#"# This file is used to configure integrations.
#
# If there is a syntax error in this file, integrations will not be loaded.
#
# Set rules to require confirmation or deny commands. If both apply, the command is denied.
#
# Rules use glob patterns for wildcard matching (https://en.wikipedia.org/wiki/Glob_(programming))
#
commands_need_confirmation:
  - "gh * delete*"
commands_deny:
  - "gh auth token*"

# GitHub integration configuration
github:
  GH_TOKEN: # <YOUR_GITHUB_TOKEN_HERE> (see https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens)
  # gh_binary_path: # Uncomment to set a custom path for the gh binary, defaults to "gh"
"#;
