pub const COMPILED_IN_INITIAL_PRIVACY_YAML: &str = r#"# Define file access levels based on patterns.
# Uses glob patterns: https://en.wikipedia.org/wiki/Glob_(programming)
# The most restrictive rule applies if a file matches multiple patterns.

file_privacy:
  only_send_to_servers_I_control:
    # Files here will only be sent to servers you control (e.g., self-hosted).
    # You can add specific files or use patterns (e.g., "*.config").
    - "*.env"
    - "*.env.*"

  blocked:
    # Files listed here are completely blocked from being accessed.
    - "*.pem"
    - "*.key"
    - "*.pub"
"#;
