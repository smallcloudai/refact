curl http://127.0.0.1:8001/v1/commit-message-from-diff -k \
  -H 'Content-Type: application/json' \
  -d '{
        "diff": "diff --git a/calculator.py b/calculator.py\nindex 1234567..89abcde 100644\n--- a/calculator.py\n+++ b/calculator.py\n@@ -1,6 +1,9 @@\n class Calculator:\n     def add(self, a, b):\n         return a + b\n\n+    def subtract(self, a, b):\n+        return a - b\n+\n import unittest\n\n class TestCalculator(unittest.TestCase):\n@@ -9,6 +12,9 @@ class TestCalculator(unittest.TestCase):\n     def test_add(self):\n         self.assertEqual(self.calc.add(2, 3), 5)\n\n+    def test_subtract(self):\n+        self.assertEqual(self.calc.subtract(5, 3), 2)\n+"
      }'

curl http://127.0.0.1:8001/v1/commit-message-from-diff -k \
  -H 'Content-Type: application/json' \
  -d '{
        "diff": "diff --git a/calculator.py b/calculator.py\nindex 1234567..89abcde 100644\n--- a/calculator.py\n+++ b/calculator.py\n@@ -1,6 +1,9 @@\n class Calculator:\n     def add(self, a, b):\n         return a + b\n\n+    def subtract(self, a, b):\n+        return a - b\n+\n import unittest\n\n class TestCalculator(unittest.TestCase):\n@@ -9,6 +12,9 @@ class TestCalculator(unittest.TestCase):\n     def test_add(self):\n         self.assertEqual(self.calc.add(2, 3), 5)\n\n+    def test_subtract(self):\n+        self.assertEqual(self.calc.subtract(5, 3), 2)\n+",
        "text": "[CI/CD] calculator features"
      }'
