pub const COMPILED_IN_INITIAL_PRIVACY_YAML: &str = r#"#
# This config file determines if Refact is allowed to read, index or send a file to remote servers.
#
# If you have a syntax error in this file, the refact-lsp process will panic and crash, because it
# can't be sure which files it can touch.
#
# Uses glob patterns: https://en.wikipedia.org/wiki/Glob_(programming)
#
# The most restrictive rule applies if a file matches multiple patterns.

privacy_rules:
    blocked:
        - */secret_project1/*            # Don't forget leading */ if you are matching directory names
        - */secret_project2/*.txt
        - *.pem

    only_send_to_servers_I_control:      # You can set up which ones you control in bring-your-own-key.yaml, otherwise you control none
        - "secret_passwords.txt"


# See unit tests in privacy.rs for more examples
"#;
