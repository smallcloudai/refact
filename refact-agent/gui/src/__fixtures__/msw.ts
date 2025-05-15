import { http, HttpResponse, type HttpHandler } from "msw";
import { EMPTY_CAPS_RESPONSE, STUB_CAPS_RESPONSE } from "./caps";
import { SYSTEM_PROMPTS } from "./prompts";
import { STUB_LINKS_FOR_CHAT_RESPONSE } from "./chat_links_response";
import {
  TOOLS,
  CHAT_LINKS_URL,
  KNOWLEDGE_CREATE_URL,
  KNOWLEDGE_SUB_URL,
  KNOWLEDGE_UPDATE_USED_URL,
} from "../services/refact/consts";
import { STUB_TOOL_RESPONSE } from "./tools_response";
import { STUB_SUB_RESPONSE, STUB_SUB_RESPONSE_WITH_STATUS } from "./knowledge";
import { GoodPollingResponse } from "../services/smallcloud";
import type { LinksForChatResponse } from "../services/refact/links";
import { SaveTrajectoryResponse } from "../services/refact/knowledge";
import { ToolConfirmationResponse } from "../services/refact";

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

export const emptyCaps: HttpHandler = http.get(
  `http://127.0.0.1:8001/v1/caps`,
  () => {
    return HttpResponse.json(EMPTY_CAPS_RESPONSE);
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
      metering_balance: 100000,
      questionnaire: {},
      refact_agent_max_request_num: 20,
      refact_agent_request_available: 20,
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

export const knowLedgeLoading: HttpHandler = http.post(
  `http://127.0.0.1:8001${KNOWLEDGE_SUB_URL}`,
  async () => {
    const encoder = new TextEncoder();
    const stream = new ReadableStream({
      start(controller) {
        // Encode the string chunks using "TextEncoder".
        STUB_SUB_RESPONSE.forEach((item) => {
          const str = `data: ${JSON.stringify(item)}\n\n`;
          controller.enqueue(encoder.encode(str));
        });

        controller.close();
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 3000));

    return new HttpResponse(stream, {
      headers: {
        "Content-Type": "text/plain",
      },
    });
  },
);

export const KnowledgeWithStatus: HttpHandler = http.post(
  `http://127.0.0.1:8001${KNOWLEDGE_SUB_URL}`,
  () => {
    const encoder = new TextEncoder();
    const stream = new ReadableStream({
      async start(controller) {
        // Encode the string chunks using "TextEncoder".
        for (const item of STUB_SUB_RESPONSE_WITH_STATUS) {
          const str = `data: ${JSON.stringify(item)}\n\n`;
          controller.enqueue(encoder.encode(str));
          await new Promise((resolve) => setTimeout(resolve, 3000)); // 1-second delay
        }

        controller.close();
      },
    });

    return new HttpResponse(stream, {
      headers: {
        "Content-Type": "text/plain",
      },
    });
  },
);

export const makeKnowledgeFromChat: HttpHandler = http.post(
  `http://127.0.0.1:8001${KNOWLEDGE_CREATE_URL}`,
  () => {
    const result: SaveTrajectoryResponse = {
      memid: "foo",
      trajectory: "something",
    };
    return HttpResponse.json(result);
  },
);

export const updateKnowledgeUse: HttpHandler = http.post(
  `http://127.0.0.1:8001${KNOWLEDGE_UPDATE_USED_URL}`,
  async (ctx) => {
    const body = await ctx.request.json();
    const memid =
      body &&
      typeof body === "object" &&
      "memid" in body &&
      typeof body.memid === "string"
        ? body.memid
        : "foo";
    const result: SaveTrajectoryResponse = {
      memid,
      trajectory: "something",
    };

    await new Promise((r) => setTimeout(r, 1000));
    return HttpResponse.json(result);
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

export const ToolConfirmation = http.post(
  "http://127.0.0.1:8001/v1/tools-check-if-confirmation-needed",
  () => {
    const response: ToolConfirmationResponse = {
      pause: false,
      pause_reasons: [],
    };

    return HttpResponse.json(response);
  },
);
