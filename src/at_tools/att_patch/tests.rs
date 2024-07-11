#[cfg(test)]
mod tests {
    use crate::at_tools::att_patch::diff_formats::UnifiedDiffFormat;
    use crate::call_validation::DiffChunk;

    #[tokio::test]
    async fn test_empty_1() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
```
Another text"#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_empty_2() {
        let input = r#""#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_empty_3() {
        let input = r#"Initial text
```diff
Another text"#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_empty_4() {
        let input = r#"Initial text
```"#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_empty_5() {
        let input = r#"Initial text
```diff
some invalid text
```
```
```diff"#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_simple_hunk_1() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-class Frog:
+class AnotherFrog:
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 5,
                line2: 5,
                lines_remove: "class Frog:\n".to_string(),
                lines_add: "class AnotherFrog:\n".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_simple_hunk_2() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
DT = 0.01

-class Frog:
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 5,
                line2: 5,
                lines_remove: "class Frog:\n".to_string(),
                lines_add: "".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_simple_hunk_3() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
DT = 0.01

class Frog:
+    # Frog class description
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 6,
                line2: 6,
                lines_remove: "".to_string(),
                lines_add: "    # Frog class description\n".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_simple_hunk_4() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
+    # Frog class description
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 1,
                line2: 1,
                lines_remove: "".to_string(),
                lines_add: "    # Frog class description\n".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_simple_hunk_5() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-    def jump(self, pond_width, pond_height):
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 22,
                line2: 22,
                lines_remove: "    def jump(self, pond_width, pond_height):\n".to_string(),
                lines_add: "".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_complex_hunk_1() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
-        elif self.x > pond_width:
+        # test1:
+        elif self.x > pond:
-            self.vx = -np.abs(self.vx)
-        if self.y < 0:
+            # what is that?
+            pass
+        if self.y > 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
-            self.vy = -np.abs(self.vy)
+            self.vx = -np.abs(self.vy)
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 15,
                line2: 17,
                lines_remove: "        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n".to_string(),
                lines_add: "        # test1:\n        elif self.x > pond:\n            # what is that?\n            pass\n        if self.y > 0:\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 20,
                line2: 20,
                lines_remove: "            self.vy = -np.abs(self.vy)\n".to_string(),
                lines_add: "            self.vx = -np.abs(self.vy)\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_complex_hunk_2() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
-        elif self.x > pond_width:
+        # test1:
+        elif self.x > pond:
-            self.vx = -np.abs(self.vx)
-        if self.y < 0:
+            # what is that?
+            pass
+        if self.y > 0:
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-            self.vy = -np.abs(self.vy)
+            self.vx = -np.abs(self.vy)
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 15,
                line2: 17,
                lines_remove: "        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n".to_string(),
                lines_add: "        # test1:\n        elif self.x > pond:\n            # what is that?\n            pass\n        if self.y > 0:\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 20,
                line2: 20,
                lines_remove: "            self.vy = -np.abs(self.vy)\n".to_string(),
                lines_add: "            self.vx = -np.abs(self.vy)\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_complex_hunk_3() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
-        elif self.x > pond_width:
+        # test1:
+        elif self.x > pond:
-            self.vx = -np.abs(self.vx)
-        if self.y < 0:
+            # what is that?
+            pass
+        if self.y > 0:
```
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-            self.vy = -np.abs(self.vy)
+            self.vx = -np.abs(self.vy)
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 15,
                line2: 17,
                lines_remove: "        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n".to_string(),
                lines_add: "        # test1:\n        elif self.x > pond:\n            # what is that?\n            pass\n        if self.y > 0:\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 20,
                line2: 20,
                lines_remove: "            self.vy = -np.abs(self.vy)\n".to_string(),
                lines_add: "            self.vx = -np.abs(self.vy)\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_complex_hunk_4() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
        self.y = np.clip(self.y, 0, pond_height)
+        # extra row 1
+        # extra row 2
+        # extra row 3
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-import numpy as np
+import numpy as np
+# extra row 1
+# extra row 2
+# extra row 3
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 27,
                line2: 27,
                lines_remove: "".to_string(),
                lines_add: "        # extra row 1\n        # extra row 2\n        # extra row 3\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 1,
                line2: 1,
                lines_remove: "import numpy as np\n".to_string(),
                lines_add: "import numpy as np\n# extra row 1\n# extra row 2\n# extra row 3\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_1() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/set_as_avatar.py
+++ tests/emergency_frog_situation/set_as_avatar.py
@@ ... @@
    """

    def __init__(self, x, y, vx, vy):
+        # extra row 1
+        # extra row 2
+        # extra row 3
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/set_as_avatar.py".to_string(),
                file_action: "edit".to_string(),
                line1: 24,
                line2: 24,
                lines_remove: "".to_string(),
                lines_add: "        # extra row 1\n        # extra row 2\n        # extra row 3\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_2() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/holiday.py
+++ tests/emergency_frog_situation/holiday.py
@@ ... @@
    frog2.jump()

-    # Third jump
+    # New Comment
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 16,
                line2: 16,
                lines_remove: "    # Third jump\n".to_string(),
                lines_add: "    # New Comment\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }
}
