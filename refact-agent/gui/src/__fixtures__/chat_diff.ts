export const CHAT_WITH_DIFF = {
  id: "9afd6fef-3e49-40df-8aca-688af3621514",
  messages: [
    {
      role: "assistant",
      content:
        "Persistence is essential in software development to ensure that data is stored and maintained even after the application is closed or the system is shut down.",
      tool_calls: null,
      finish_reason: "stop",
      tool_call_id: "",
    },
    {
      role: "context_file",
      content:
        '[{"file_name": "hibernate-orm/hibernate-core/src/test/java/org/hibernate/orm/test/id/usertype/UserTypeComparableIdTest.java", "line1": 1, "line2": 228, "file_content": "/*\\n * Hibernate, Relational Persistence for Idiomatic Java\\n *\\n * License: GNU Lesser General Public License (LGPL), version 2.1 or later.\\n * See the lgpl.txt"}]',
      tool_calls: null,
      finish_reason: "",
      tool_call_id: "",
    },
    {
      role: "diff",
      content:
        '[{"file_name": "file1.py", "file_action": "edit", "line1": 5, "line2": 6, "lines_remove": "def f(x: int):\\n    return x*2\\n", "lines_add": "def f(x: float):\\n    return x*3\\n"}, {"file_name": "file2.py", "file_action": "new", "lines_add": "def main():\\n    file1.f(6)\\n"}]',
      tool_calls: null,
      finish_reason: "",
      tool_call_id: "",
    },
  ],
  title: "Chat with diff",
  model: "gpt-3.5-turbo",
};
