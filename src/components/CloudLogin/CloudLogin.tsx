import { Button, Flex, Text, TextField } from "@radix-ui/themes";
import { Checkbox } from "../Checkbox";
import { useCallback, useEffect, useRef, useState } from "react";

export interface CloudLoginProps {
  goBack: () => void;
  next: (apiKey: string, sendCorrectedCodeSnippets: boolean) => void;
  openExternal: (url: string) => void;
}

interface OkResponse {
  retcode: "OK";
  secret_key: string;
}

function isOkResponse(json: unknown): json is OkResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("retcode" in json)) return false;
  if (json.retcode !== "OK") return false;
  if (!("secret_key" in json)) return false;
  if (typeof json.secret_key !== "string") return false;
  return true;
}

export const CloudLogin: React.FC<CloudLoginProps> = ({
  goBack,
  next,
  openExternal,
}: CloudLoginProps) => {
  const [sendCorrectedCodeSnippets, setSendCorrectedCodeSnippets] =
    useState(false);
  const [apiKey, setApiKey] = useState("");
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(false);
  const loginTicket = useRef("");
  const interval = useRef<NodeJS.Timeout | undefined>(undefined);
  const input = useRef<HTMLInputElement>(null);

  const login = useCallback(() => {
    setLoading(true);

    const newLoginTicket =
      Math.random().toString(36).substring(2, 15) +
      "-" +
      Math.random().toString(36).substring(2, 15);
    loginTicket.current = newLoginTicket;
    openExternal(
      `https://refact.smallcloud.ai/authentication?token=${newLoginTicket}&utm_source=plugin&utm_medium=vscode&utm_campaign=login`,
    );

    if (interval.current !== undefined) {
      clearInterval(interval.current);
    }

    interval.current = setInterval(() => {
      if (loginTicket.current !== newLoginTicket) {
        return;
      }
      if (apiKey === "") {
        const fetchApiKey = async () => {
          const url =
            "https://www.smallcloud.ai/v1/streamlined-login-recall-ticket";
          const headers = {
            "Content-Type": "application/json",
            Authorization: `codify-${newLoginTicket}`,
          };
          const init: RequestInit = {
            method: "GET",
            redirect: "follow",
            cache: "no-cache",
            referrer: "no-referrer",
            headers,
          };
          const response = await fetch(url, init);
          if (!response.ok) {
            // eslint-disable-next-line no-console
            console.error(`Unable to recall login ticket: ${response.status}`);
            return;
          }

          const json: unknown = await response.json();
          if (isOkResponse(json)) {
            if (interval.current) {
              setApiKey(json.secret_key);
              setLoading(false);
              clearInterval(interval.current);
              interval.current = undefined;
            }
          }
        };
        void fetchApiKey();
      }
    }, 5000);
  }, [loginTicket, apiKey, openExternal, setApiKey]);

  useEffect(() => {
    setError(false);
    setLoading(false);
    if (interval.current) {
      interval.current = undefined;
    }
  }, [apiKey]);

  useEffect(() => {
    const { current } = input;
    if (current === null || !error) {
      return;
    }
    current.focus();
  }, [error]);

  useEffect(() => {
    const { current } = input;
    if (current === null) {
      return;
    }

    if (loading) {
      const loadingText = "Fetching API Key ";
      const animationFrames = ["/", "|", "\\", "-"];
      let index = 0;

      const interval = setInterval(() => {
        current.placeholder = `${loadingText} ${animationFrames[index]}`;
        index = (index + 1) % animationFrames.length;
      }, 100);

      return () => {
        clearInterval(interval);
      };
    } else {
      current.placeholder = "";
    }
  }, [input, loading]);

  const canSubmit = Boolean(apiKey);
  const onSubmit = () => {
    if (!canSubmit) {
      setError(true);
      return;
    }
    next(apiKey, sendCorrectedCodeSnippets);
  };

  return (
    <Flex direction="column" gap="2" maxWidth="540px" m="8px">
      <Text weight="bold" size="4">
        Cloud inference
      </Text>
      <Text size="2">Quick login via website:</Text>
      <Button onClick={login}>Login / Create Account</Button>
      <Text size="2" mt="2">
        Alternatively, paste an existing Refact API key here:
      </Text>
      <TextField.Root
        ref={input}
        value={apiKey}
        onChange={(event) => setApiKey(event.target.value)}
        color={error ? "red" : undefined}
        onBlur={() => setError(false)}
      />
      {error && (
        <Text size="2" color="red">
          Please Login / Create Account or enter API key
        </Text>
      )}
      <Text size="2" mt="4">
        Help Refact collect a dataset of corrected code completions! This will
        help to improve code suggestions more to your preferences, and it also
        will improve code suggestions for everyone else. Hey, we&#39;re not an
        evil corporation!
      </Text>
      <Checkbox
        checked={sendCorrectedCodeSnippets}
        onCheckedChange={(value) =>
          setSendCorrectedCodeSnippets(Boolean(value))
        }
      >
        Send corrected code snippets.
      </Checkbox>
      <Text size="2">
        Basic telemetry is always on when using cloud inference, but it only
        sends errors and counters.{" "}
        <a href="https://github.com/smallcloudai/refact-lsp/blob/main/README.md#telemetry">
          How telemetry works in open source refact-lsp
        </a>
      </Text>
      <Flex gap="2">
        <Button variant="outline" mr="auto" onClick={goBack}>
          {"< Back"}
        </Button>
        <Button variant="outline" ml="auto" type="submit" onClick={onSubmit}>
          {"Next >"}
        </Button>
      </Flex>
    </Flex>
  );
};
