import { http, HttpResponse, type HttpHandler } from "msw";
import { STUB_CAPS_RESPONSE } from "./caps";
import { SYSTEM_PROMPTS } from "./prompts";
import { STUB_LINKS_FOR_CHAT_RESPONSE } from "./chat_links_response";
import {
  AT_TOOLS_AVAILABLE_URL,
  CHAT_LINKS_URL,
} from "../services/refact/consts";
import { STUB_TOOL_RESPONSE } from "./tools_response";
import { GoodPollingResponse } from "../services/smallcloud";

export const goodPing: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/ping",
  () => {
    return HttpResponse.text("pong");
  },
);

export const goodCaps: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/caps",
  () => {
    return HttpResponse.json(STUB_CAPS_RESPONSE);
  },
);

export const noTools: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/tools",
  () => {
    return HttpResponse.json([]);
  },
);

export const goodPrompts: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/customization",
  () => {
    return HttpResponse.json({ system_prompts: SYSTEM_PROMPTS });
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
      metering_balance: -100000,
      questionnaire: {},
      refact_agent_max_request_num: 20,
      refact_agent_request_available: null,
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
      refact_agent_max_request_num: 20,
      refact_agent_request_available: 5,
    });
  },
);

export const chatLinks: HttpHandler = http.post(
  `http://127.0.0.1:8001${CHAT_LINKS_URL}`,
  () => {
    return HttpResponse.json(STUB_LINKS_FOR_CHAT_RESPONSE);
  },
);

export const goodTools: HttpHandler = http.get(
  `http://127.0.0.1:8001${AT_TOOLS_AVAILABLE_URL}`,
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
      questionnaire: {},
      refact_agent_max_request_num: 20,
      refact_agent_request_available: null,
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
