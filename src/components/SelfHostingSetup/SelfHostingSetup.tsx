import { Button, Flex, Text, TextField } from "@radix-ui/themes";
import { useEffect, useRef, useState } from "react";

export interface SelfHostingSetupProps {
  goBack: () => void;
  next: (endpointAddress: string) => void;
}

export const SelfHostingSetup: React.FC<SelfHostingSetupProps> = ({
  goBack,
  next,
}: SelfHostingSetupProps) => {
  const [endpoint, setEndpoint] = useState("");
  const [error, setError] = useState(false);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const { current } = input;
    if (current === null || !error) {
      return;
    }
    current.focus();
  }, [error]);

  useEffect(() => {
    setError(false);
  }, [endpoint]);

  const canSubmit = Boolean(endpoint);
  const onSubmit = () => {
    if (!canSubmit) {
      setError(true);
      return;
    }

    next(endpoint);
  };

  return (
    <Flex direction="column" gap="2" maxWidth="540px" m="8px">
      <Text size="2">
        A great option for self-hosting is{" "}
        <a href="https://github.com/smallcloudai/refact/">Refact docker</a>. It
        can serve completion and chat models, has graphical user interface to
        set it up, and it can fine-tune code on your codebase. A typical
        endpoint address is http://127.0.0.1:8008/
        <br />
        But this plugin might work with a variety of servers, report your
        experience on discord!
      </Text>
      <Text size="2">Endpoint Address</Text>
      <TextField.Root
        value={endpoint}
        ref={input}
        onChange={(event) => setEndpoint(event.target.value)}
        color={error ? "red" : undefined}
        onBlur={() => setError(false)}
      />
      {error && (
        <Text size="2" color="red">
          Please enter endpoint
        </Text>
      )}
      <Flex gap="2">
        <Button variant="outline" mr="auto" onClick={goBack}>
          {"< Back"}
        </Button>
        <Button ml="auto" onClick={onSubmit}>
          {"Save"}
        </Button>
      </Flex>
    </Flex>
  );
};
