export type Workspace = {
  workspace_id: number;
  workspace_name: string;
};

export type User = {
  retcode: string;
  account: string;
  inference_url: string;
  inference: string;
  metering_balance: number;
  workspaces: Workspace[];
  questionnaire: false | Record<string, string>;
};

export function isUser(json: unknown): json is User {
  return (
    typeof json === "object" &&
    json !== null &&
    "retcode" in json &&
    typeof json.retcode === "string" &&
    "account" in json &&
    typeof json.account === "string" &&
    "inference_url" in json &&
    typeof json.inference_url === "string" &&
    "inference" in json &&
    typeof json.inference === "string" &&
    "metering_balance" in json &&
    typeof json.metering_balance === "number" &&
    "workspaces" in json &&
    Array.isArray(json.workspaces)
  );
}

export type GoodPollingResponse = User & {
  secret_key: string;
  tooltip_message: string;
  login_message: string;
  "longthink-filters": unknown[];
  "longthink-functions-today": Record<string, LongThinkFunction>;
  "longthink-functions-today-v2": Record<string, LongThinkFunction>;
};

export type DetailedUserResponse = User & {
  tooltip_message: string;
  login_message: string;
};

export function isGoodResponse(json: unknown): json is GoodPollingResponse {
  if (!isUser(json)) return false;
  return "secret_key" in json && typeof json.secret_key === "string";
}

export function isUserWithLoginMessage(
  json: unknown,
): json is DetailedUserResponse {
  if (!isUser(json)) return false;
  return (
    "tooltip_message" in json &&
    typeof json.tooltip_message === "string" &&
    "login_message" in json &&
    typeof json.login_message === "string"
  );
}

export type BadResponse = {
  human_readable_message: string;
  retcode: "FAILED";
};

export type StreamedLoginResponse = DetailedUserResponse | BadResponse;

export type LongThinkFunction = {
  label: string;
  model: string;
  selected_lines_min: number;
  selected_lines_max: number;
  metering: number;
  "3rd_party": boolean;
  supports_highlight: boolean;
  supports_selection: boolean;
  always_visible: boolean;
  mini_html: string;
  likes: number;
  supports_languages: string;
  is_liked: boolean;
  function_highlight: string;
  function_selection: string;
};

export type RadioOptions = {
  title: string;
  value: string;
};

export interface SurveyQuestion {
  type: string;
  name: string;
  question: string;
}

export function isSurveyQuestion(json: unknown): json is SurveyQuestion {
  if (!json) return false;
  if (typeof json !== "object") return false;
  return (
    "type" in json &&
    typeof json.type === "string" &&
    "name" in json &&
    typeof json.name === "string" &&
    "question" in json &&
    typeof json.question === "string"
  );
}

export interface RadioQuestion extends SurveyQuestion {
  type: "radio";
  options: RadioOptions[];
}

export function isRadioQuestion(
  question: SurveyQuestion,
): question is RadioQuestion {
  return question.type === "radio";
}

export type SurveyQuestions = (RadioQuestion | SurveyQuestion)[];

export function isSurveyQuestions(json: unknown): json is SurveyQuestions {
  if (!Array.isArray(json)) return false;
  return json.every(isSurveyQuestion);
}

export type EmailLinkResponse =
  | {
      retcode: "OK";
      status: "sent";
    }
  | {
      retcode: "OK";
      status: "not_logged_in";
    }
  | {
      retcode: "OK";
      status: "user_logged_in";
      key: string;
    };

export function isEmailLinkResponse(json: unknown): json is EmailLinkResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  return (
    "retcode" in json &&
    typeof json.retcode === "string" &&
    "status" in json &&
    typeof json.status === "string"
  );
}
