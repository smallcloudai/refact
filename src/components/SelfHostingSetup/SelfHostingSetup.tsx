import { Button, Flex, Text, TextField } from "@radix-ui/themes";
import { useState } from "react";

export interface SelfHostingSetupProps {
  goBack: () => void;
  next: (endpointAddress: string) => void;
}

export const SelfHostingSetup: React.FC<SelfHostingSetupProps> = ({
  goBack,
  next,
}: SelfHostingSetupProps) => {
  const [endpoint, setEndpoint] = useState("");

  const canSubmit = Boolean(endpoint);
  const onSubmit = () => {
    if (canSubmit) {
      next(endpoint);
    }
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
        onChange={(event) => setEndpoint(event.target.value)}
      />
      <Flex gap="2">
        <Button variant="outline" mr="auto" onClick={goBack}>
          Back
        </Button>
        <Button
          variant="outline"
          ml="auto"
          disabled={!canSubmit}
          onClick={onSubmit}
        >
          Next
        </Button>
      </Flex>
    </Flex>
  );
};
