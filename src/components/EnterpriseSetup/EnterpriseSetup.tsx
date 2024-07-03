import { Button, Flex, Text, TextField } from "@radix-ui/themes";
import { useState } from "react";

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

  const canSubmit = Boolean(endpoint && apiKey);
  const onSubmit = () => {
    if (canSubmit) {
      next(endpoint, apiKey);
    }
  };

  return (
    <Flex direction="column" gap="2">
      <Text size="2">
        You should have corporate endpoint URL and personal API key. Please
        contact your system administrator.
      </Text>
      <Text size="2">Endpoint Address</Text>
      <TextField.Root
        placeholder="http://x.x.x.x:8008/"
        value={endpoint}
        onChange={(event) => setEndpoint(event.target.value)}
      />
      <Text size="2">API Key</Text>
      <TextField.Root
        value={apiKey}
        onChange={(event) => setApiKey(event.target.value)}
      />
      <Flex gap="2">
        <Button variant="outline" mr="auto" onClick={goBack}>
          Back
        </Button>
        <Button
          variant="outline"
          ml="auto"
          type="submit"
          disabled={!canSubmit}
          onClick={onSubmit}
        >
          Next
        </Button>
      </Flex>
    </Flex>
  );
};
