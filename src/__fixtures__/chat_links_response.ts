import { LinksForChatResponse } from "../services/refact/links";

export const STUB_LINKS_FOR_CHAT_RESPONSE: LinksForChatResponse = {
  links: [
    {
      text: "Save and return",
      action: "patch-all",
      goto: "SETTINGS:/path/to/config/file.yaml",
      link_tooltip: "",
    },
    {
      text: "Can you fix it?",
      action: "follow-up",
      link_tooltip: "a nice tool tip message",
    },
    { text: 'git commit -m "message"', action: "commit", link_tooltip: "" },
    { text: "Save and return", goto: "SETTINGS:postgres", link_tooltip: "" },
    {
      text: "Investigate Project",
      action: "summarize-project",
      link_tooltip: "",
    },
  ],
};
