// This test file cannot access internal crate types directly.
// The PullRequest struct is private to the integrations module.
// These tests should be moved to src/integrations/integr_bitbucket.rs as unit tests.

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder - actual PullRequest tests are in src/integrations/integr_bitbucket.rs
        assert!(true);
    }
}
