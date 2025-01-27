import { LinksForChatResponse } from "../services/refact/links";

export const STUB_LINKS_FOR_CHAT_RESPONSE: LinksForChatResponse = {
  uncommited_changes_warning:
    "You have uncommitted changes:\n```\nIn project refact-lsp: A tests/emergency_frog_situation/.refact/project_summary.yaml, M tests/emergency_frog_situation/frog.py, M tests/emergency_frog_situation/jump_to_conclusions.py, ...\n```\n⚠️ You might have a problem rolling back agent's changes.",

  links: [
    {
      link_text: "Save and return",
      link_action: "patch-all",
      link_goto: "SETTINGS:/path/to/config/file.yaml",
      link_tooltip: "",
    },
    {
      link_text: "Save and Continue",
      link_action: "patch-all",
      link_goto: "NEWCHAT",
      link_tooltip: "",
    },
    {
      link_text: "Can you fix it?",
      link_action: "follow-up",
      link_tooltip: "a nice tool tip message",
    },
    // { text: 'git commit -m "message"', action: "commit", link_tooltip: "" },
    // { text: "Save and return", goto: "SETTINGS:postgres", link_tooltip: "" },
    {
      link_text: "Investigate Project",
      link_action: "summarize-project",
      link_tooltip: "",
    },
    {
      link_action: "post-chat",
      link_text: "Stop recommending integrations",
      link_tooltip: "",
      link_payload: {
        chat_meta: {
          chat_id: "",
          chat_remote: false,
          chat_mode: "CONFIGURE",
          current_config_file:
            "/Users/kot/code_aprojects/demotest/.refact/project_summary.yaml",
        },
        messages: [
          {
            role: "user",
            content:
              "Make recommended_integrations an empty list, follow the system prompt.",
          },
        ],
      },
    },
    // {
    //   text: "long long long long long long long long long long long long long long long long long long ",
    //   action: "summarize-project",
    //   link_tooltip: "",
    // },
    {
      link_action: "commit",
      link_text: "Commit 4 files in `refact-lsp`",
      link_goto: "LINKS_AGAIN",
      link_tooltip:
        'git commmit -m "Add build script and test files for Docker image deployment and output generation..."\nA build-remote.sh\nA long-array.py\nA long-output.py\nA test.py',
      link_payload: {
        project_path: "file:///Users/humbertoyusta/refact/refact-lsp",
        commit_message:
          "Add build script and test files for Docker image deployment and output generation\n\nIntroduced `build-remote.sh` to streamline the process of building a Docker image and deploying it to a remote server. This script automates the image creation, temporary container management, and binary transfer steps, improving efficiency and reducing manual errors. Additionally, added `long-array.py`, `long-output.py`, and `test.py` to facilitate testing and output generation scenarios, ensuring the system can handle large data sets and multiple output streams effectively.",
        file_changes: [
          {
            path: "build-remote.sh",
            status: "ADDED",
          },
          {
            path: "long-array.py",
            status: "ADDED",
          },
          {
            path: "long-output.py",
            status: "ADDED",
          },
          {
            path: "test.py",
            status: "ADDED",
          },
        ],
      },
    },
  ],
};
