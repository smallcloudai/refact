import { Button, Flex, Text, TextField } from "@radix-ui/themes";
import { useEffect, useRef, useState } from "react";
import { ChevronLeftIcon } from "@radix-ui/react-icons";

export interface EnterpriseSetupProps {
  goBack: () => void;
  next: (endpointAddress: string, apiKey: string) => void;
}

export const EnterpriseSetup: React.FC<EnterpriseSetupProps> = ({
  goBack,
  next,
}: EnterpriseSetupProps) => {
  const [endpoint, setEndpoint] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [endpointError, setEndpointError] = useState(false);
  const [apiKeyError, setApiKeyError] = useState(false);
  const endpointInput = useRef<HTMLInputElement>(null);
  const apiKeyInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setApiKeyError(false);
  }, [apiKey]);

  useEffect(() => {
    setEndpointError(false);
  }, [endpoint]);

  useEffect(() => {
    const { current } = endpointInput;
    if (current === null || !endpointError) {
      return;
    }
    current.focus();
  }, [endpointError]);

  useEffect(() => {
    const { current } = apiKeyInput;
    if (current === null || !apiKeyError) {
      return;
    }
    current.focus();
  }, [apiKeyError]);

  const onSubmit = () => {
    if (!endpoint) {
      setEndpointError(true);
      return;
    }
    if (!apiKey) {
      setApiKeyError(true);
      return;
    }
    next(endpoint, apiKey);
  };

  return (
    <Flex
      direction="column"
      gap="2"
      maxWidth="540px"
      m="8px"
      style={{ alignSelf: "center" }}
    >
      <Text size="2">
        You should have corporate endpoint URL and personal API key. Please
        contact your system administrator.
      </Text>
      <Text size="2">Endpoint Address</Text>
      <TextField.Root
        ref={endpointInput}
        placeholder="http://x.x.x.x:8008/"
        value={endpoint}
        onChange={(event) => setEndpoint(event.target.value)}
        color={endpointError ? "red" : undefined}
        onBlur={() => setEndpointError(false)}
      />
      {endpointError && (
        <Text size="2" color="red">
          Please enter endpoint
        </Text>
      )}
      <Text size="2">API Key</Text>
      <TextField.Root
        ref={apiKeyInput}
        value={apiKey}
        onChange={(event) => setApiKey(event.target.value)}
        color={apiKeyError ? "red" : undefined}
        onBlur={() => setApiKeyError(false)}
      />
      {apiKeyError && (
        <Text size="2" color="red">
          Please enter API key
        </Text>
      )}
      <Flex gap="2">
        <Button
          color="gray"
          highContrast
          variant="outline"
          mr="auto"
          onClick={goBack}
        >
          <ChevronLeftIcon />
          {"Back"}
        </Button>
        <Button
          color="gray"
          highContrast
          variant="solid"
          ml="auto"
          type="submit"
          onClick={onSubmit}
        >
          {"Save"}
        </Button>
      </Flex>
    </Flex>
  );
};
