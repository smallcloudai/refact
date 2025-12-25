import { http, HttpResponse, type HttpHandler } from "msw";
import { EMPTY_CAPS_RESPONSE, STUB_CAPS_RESPONSE } from "./caps";
import { SYSTEM_PROMPTS } from "./prompts";
import { STUB_LINKS_FOR_CHAT_RESPONSE } from "./chat_links_response";
import {
  TOOLS,
  CHAT_LINKS_URL,
} from "../services/refact/consts";
import { STUB_TOOL_RESPONSE } from "./tools_response";
import { GoodPollingResponse } from "../services/smallcloud/types";
import type { LinksForChatResponse } from "../services/refact/links";
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

export const goodCapsWithKnowledgeFeature: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/caps",
  () => {
    return HttpResponse.json({
      ...STUB_CAPS_RESPONSE,
      metadata: { features: ["knowledge"] },
    });
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

export const emptyTrajectories: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/trajectories",
  () => {
    return HttpResponse.json([]);
  },
);

export const trajectoryGet: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/trajectories/:id",
  () => {
    return HttpResponse.json({ status: "not_found" }, { status: 404 });
  },
);

export const trajectorySave: HttpHandler = http.put(
  "http://127.0.0.1:8001/v1/trajectories/:id",
  () => {
    return HttpResponse.json({ status: "ok" });
  },
);

export const trajectoryDelete: HttpHandler = http.delete(
  "http://127.0.0.1:8001/v1/trajectories/:id",
  () => {
    return HttpResponse.json({ status: "ok" });
  },
);

// Chat Session (Stateless Trajectory UI) handlers
export const chatSessionSubscribe: HttpHandler = http.get(
  "http://127.0.0.1:8001/v1/chats/subscribe",
  () => {
    // Return an SSE stream that immediately closes (no events)
    const encoder = new TextEncoder();
    const stream = new ReadableStream({
      start(controller) {
        // Send a comment to keep connection alive, then close
        controller.enqueue(encoder.encode(": keep-alive\n\n"));
        // Don't close - let the client handle disconnection
      },
    });
    return new HttpResponse(stream, {
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        "Connection": "keep-alive",
      },
    });
  },
);

export const chatSessionCommand: HttpHandler = http.post(
  "http://127.0.0.1:8001/v1/chats/:id/commands",
  () => {
    return HttpResponse.json({ status: "queued" });
  },
);

export const chatSessionAbort: HttpHandler = http.post(
  "http://127.0.0.1:8001/v1/chats/:id/abort",
  () => {
    return HttpResponse.json({ status: "ok" });
  },
);
