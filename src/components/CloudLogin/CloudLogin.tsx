import { Button, Flex, Text, TextField } from "@radix-ui/themes";
import { Checkbox } from "../Checkbox";
import { useEffect, useRef, useState } from "react";

export interface CloudLoginProps {
  loading: boolean;
  apiKey: string;
  setApiKey: (value: string) => void;
  goBack: () => void;
  next: (apiKey: string, sendCorrectedCodeSnippets: boolean) => void;
  login: () => void;
}

export const CloudLogin: React.FC<CloudLoginProps> = ({
  loading,
  apiKey,
  setApiKey,
  goBack,
  next,
  login,
}: CloudLoginProps) => {
  const [sendCorrectedCodeSnippets, setSendCorrectedCodeSnippets] =
    useState(false);
  const [error, setError] = useState(false);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setError(false);
  }, [apiKey]);

  useEffect(() => {
    const current = input.current;
    if (current === null) {
      return;
    }
    current.focus();
  }, [error]);

  useEffect(() => {
    const current = input.current;
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
        <Text size="2" mt="4" color="red">
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
          Back
        </Button>
        <Button variant="outline" ml="auto" type="submit" onClick={onSubmit}>
          Next
        </Button>
      </Flex>
    </Flex>
  );
};
