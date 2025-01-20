import { http, HttpResponse } from "msw";
import { QUESTIONS_STUB } from "../__fixtures__";
import { render } from "../utils/test-utils";
import { describe, expect, test } from "vitest";
import {
  server,
  goodPrompts,
  goodCaps,
  noTools,
  noCommandPreview,
  noCompletions,
  goodPing,
  goodUser,
  chatLinks,
} from "../utils/mockServer";
import { InnerApp } from "../features/App";

const userMock = http.get(
  "https://www.smallcloud.ai/v1/login",
  () => {
    return HttpResponse.json({
      retcode: "OK",
      account: "party@refact.ai",
      inference_url: "https://www.smallcloud.ai/v1",
      inference: "PRO",
      metering_balance: -100000,
      questionnaire: false,
      refact_agent_max_request_num: 20,
      refact_agent_request_available: null,
    });
  },
  // TODO: if once if true, it still runs twice without refact_agent_max_request_num & refact_agent_request_available
  // { once: true },
);

const questionnaireMock = http.get(
  "https://www.smallcloud.ai/v1/questionnaire",
  () => {
    return HttpResponse.json(QUESTIONS_STUB);
  },
);

const saveQuestionnaireMock = http.post(
  "https://www.smallcloud.ai/v1/save-questionnaire",
  () => {
    return HttpResponse.json({ retcode: "OK" });
  },
);

describe("Start a new chat", () => {
  test("User survey should open when 'questionnaire` is false", async () => {
    server.use(
      goodPing,
      goodCaps,
      goodPrompts,
      noTools,
      noCommandPreview,
      noCompletions,
      userMock,
      goodUser,
      questionnaireMock,
      saveQuestionnaireMock,
      chatLinks,
    );

    const { user, ...app } = render(<InnerApp />, {
      preloadedState: {
        pages: [{ name: "history" }],
        config: {
          apiKey: "test",
          lspPort: 8001,
          themeProps: {},
          host: "vscode",
          addressURL: "Refact",
        },
      },
    });

    await new Promise((r) => setTimeout(r, 1000));

    expect(app.getByText(QUESTIONS_STUB[0].question)).not.toBeNull();

    const option = app.getByText("Reddit");
    await user.click(option);

    const submit = app.getByText("Submit");
    await user.click(submit);

    expect(app.queryByText(QUESTIONS_STUB[0].question)).toBeNull();

    expect(app.queryAllByText(/Thank you/i)).not.toBeNull();
  });
});
