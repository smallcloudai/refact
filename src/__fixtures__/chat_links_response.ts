import { LinksForChatResponse } from "../services/refact/links";

export const STUB_LINKS_FOR_CHAT_RESPONSE: LinksForChatResponse = {
  links: [
    {
      text: "Save and return",
      action: "patch-all",
      goto: "SETTINGS:/path/to/config/file.yaml",
    },
    { text: "Can you fix it?", action: "follow-up" },
    { text: 'git commit -m "message"', action: "commit" },
    { text: "Save and return", goto: "SETTINGS:postgres" },
    { text: "Investigate Project", action: "summarize-project" },
  ],
};
