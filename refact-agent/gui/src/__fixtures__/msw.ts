import { http, HttpResponse, type HttpHandler } from "msw";
import { STUB_LINKS_FOR_CHAT_RESPONSE } from "./chat_links_response";
import { TOOLS, CHAT_LINKS_URL } from "../services/refact/consts";
import { STUB_TOOL_RESPONSE } from "./tools_response";
import { GoodPollingResponse } from "../services/smallcloud/types";
import type { LinksForChatResponse } from "../services/refact/links";

export const goodPing: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/ping",
  () => {
    return HttpResponse.text("pong");
  },
);

export const noTools: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/tools",
  () => {
    return HttpResponse.json([]);
  },
);

export const noCompletions: HttpHandler = http.post(
  "http://127.0.0.1:8001/v1/at-command-completion",
  () => {
    return HttpResponse.json({
      completions: [],
      replace: [0, 0],
      is_cmd_executable: false,
    });
  },
);

export const noCommandPreview: HttpHandler = http.post(
  "http://127.0.0.1:8001/v1/at-command-preview",
  () => {
    return HttpResponse.json({
      messages: [],
    });
  },
);

export const goodUser: HttpHandler = http.get(
  "https://www.smallcloud.ai/v1/login",
  () => {
    return HttpResponse.json({
      retcode: "OK",
      account: "party@refact.ai",
      inference_url: "https://www.smallcloud.ai/v1",
      inference: "PRO",
      metering_balance: 100000,
      questionnaire: {},
      refact_agent_max_request_num: 20,
      refact_agent_request_available: 20,
      workspaces: [],
    });
  },
);

export const nonProUser: HttpHandler = http.get(
  "https://www.smallcloud.ai/v1/login",
  () => {
    return HttpResponse.json({
      retcode: "OK",
      account: "party@refact.ai",
      inference_url: "https://www.smallcloud.ai/v1",
      inference: "FREE",
      metering_balance: -100000,
      questionnaire: {},
      workspaces: [],
    });
  },
);

export const chatLinks: HttpHandler = http.post(
  `http://127.0.0.1:8001${CHAT_LINKS_URL}`,
  () => {
    return HttpResponse.json(STUB_LINKS_FOR_CHAT_RESPONSE);
  },
);

export const noChatLinks: HttpHandler = http.post(
  `http://127.0.0.1:8001${CHAT_LINKS_URL}`,
  () => {
    const res: LinksForChatResponse = {
      uncommited_changes_warning: "",
      new_chat_suggestion: false,
      links: [],
    };
    return HttpResponse.json(res);
  },
);

export const goodTools: HttpHandler = http.get(
  `http://127.0.0.1:8001${TOOLS}`,
  () => {
    return HttpResponse.json(STUB_TOOL_RESPONSE);
  },
);

export const loginPollingGood: HttpHandler = http.get(
  "https://www.smallcloud.ai/v1/streamlined-login-recall-ticket",
  () => {
    const result: GoodPollingResponse = {
      retcode: "OK",
      account: "party@refact.ai",
      inference_url: "https://www.smallcloud.ai/v1",
      inference: "PRO",
      metering_balance: -100000,
      // workspaces: [],
      questionnaire: {},
      secret_key: "shhhhhhhhh",
      tooltip_message: "",
      login_message: "",
      "longthink-filters": [],
      "longthink-functions-today": {},
      "longthink-functions-today-v2": {},
    };
    return HttpResponse.json(result);
  },
);

export const loginPollingWaiting: HttpHandler = http.get(
  "https://www.smallcloud.ai/v1/streamlined-login-recall-ticket",
  () => {
    const result = { human_readable_message: "", retcode: "FAILED" };
    return HttpResponse.json(result);
  },
);

export const emailLogin: HttpHandler = http.get(
  "https://www.smallcloud.ai/plugin-magic-link/*",
  async function* () {
    let count = 0;

    await new Promise((resolve) => setTimeout(resolve, 500));
    yield HttpResponse.json({
      retcode: "OK",
      status: "sent",
    });

    while (count < 5) {
      count++;
      yield HttpResponse.json({
        retcode: "OK",
        status: "not_logged_in",
      });
    }

    yield HttpResponse.json({
      retcode: "OK",
      status: "user_logged_in",
      key: "1234567890",
    });
  },
);

export const telemetryChat = http.post(
  `http://127.0.0.1:8001/v1/telemetry-chat`,
  () => {
    return HttpResponse.json({
      retcode: "OK",
      status: "sent",
    });
  },
);

export const telemetryNetwork = http.post(
  `http://127.0.0.1:8001/v1/telemetry-network`,
  () => {
    return HttpResponse.json({
      retcode: "OK",
      status: "sent",
    });
  },
);
