#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize_pull_request() {
        let json = json!({
            "id": 1,
            "title": "Test PR",
            "author": {
                "display_name": "Test User"
            },
            "state": "OPEN",
            "created_on": "2023-01-01T12:00:00Z"
        });
        let pr: PullRequest = serde_json::from_value(json).unwrap();
        assert_eq!(pr.id, 1);
        assert_eq!(pr.title, "Test PR");
        assert_eq!(pr.author.display_name, "Test User");
    }
}
