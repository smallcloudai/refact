import React, { useCallback } from "react";
import {
  Flex,
  Box,
  Button,
  Text,
  Separator,
  TextField,
  Container,
  Heading,
} from "@radix-ui/themes";
import { GitHubLogoIcon } from "@radix-ui/react-icons";
import { GoogleIcon } from "../../images/GoogleIcon";
import { Accordion } from "../../components/Accordion";
import { useEventsBusForIDE, useLogin, useEmailLogin } from "../../hooks";

export const LoginPage: React.FC = () => {
  const { loginWithProvider, polling, cancelLogin } = useLogin();
  const { setupHost } = useEventsBusForIDE();
  const { emailLogin, emailLoginResult, emailLoginAbort } = useEmailLogin();

  const emailIsLoading = React.useMemo(() => {
    if (
      emailLoginResult.isSuccess &&
      emailLoginResult.data.status !== "user_logged_in"
    ) {
      return true;
    }
    return emailLoginResult.isLoading;
  }, [
    emailLoginResult.data?.status,
    emailLoginResult.isLoading,
    emailLoginResult.isSuccess,
  ]);

  const isLoading = React.useMemo(() => {
    if (polling.isLoading || polling.isFetching) return true;
    return emailIsLoading;
  }, [polling, emailIsLoading]);

  const onCancel = useCallback(() => {
    try {
      cancelLogin.current();
      emailLoginAbort();
    } catch {
      // no-op
    }
  }, [cancelLogin, emailLoginAbort]);

  return (
    <Container>
      <Heading align="center" as="h2" size="6" my="6">
        Login to Refact.ai
      </Heading>
      <Accordion.Root
        type="single"
        defaultValue={"cloud"}
        disabled={isLoading}
        collapsible
      >
        <Accordion.Item value="cloud">
          <Accordion.Trigger>Refact Cloud</Accordion.Trigger>
          <Accordion.Content>
            <Box>
              <Text size="2">
                <ul>
                  <li>
                    Chat with your codebase powered by top models (e.g. Claude
                    3.5 Sonnet & GPT-4o with 32k context).
                  </li>
                  <li>Unlimited Code Completions (powered by Qwen2.5).</li>
                  <li>Codebase-aware vector database (RAG).</li>
                  <li>
                    Agentic features: browser use, database connect, debugger,
                    shell commands, etc.
                  </li>
                </ul>
              </Text>
            </Box>
            <Separator size="4" my="4" />
            <Flex direction="column" gap="3" align="center">
              <Button
                onClick={() => {
                  onCancel();
                  loginWithProvider("google");
                }}
                disabled={isLoading}
              >
                <GoogleIcon width="15" height="15" /> Continue with Google
              </Button>
              <Button
                onClick={() => {
                  onCancel();
                  loginWithProvider("github");
                }}
                disabled={isLoading}
              >
                <GitHubLogoIcon width="15" height="15" /> Continue with GitHub
              </Button>

              <Text>or</Text>

              <Flex asChild direction="column" gap="3">
                <form
                  onSubmit={(event) => {
                    event.preventDefault();
                    if (isLoading) return;
                    const formData = new FormData(event.currentTarget);
                    const email = formData.get("email");
                    if (typeof email === "string") {
                      emailLogin(email);
                    }
                  }}
                >
                  <TextField.Root
                    placeholder="Email Address"
                    type="email"
                    name="email"
                    required
                    disabled={isLoading}
                  />
                  <Button
                    type="submit"
                    loading={emailIsLoading}
                    disabled={isLoading}
                  >
                    Send magic link
                  </Button>{" "}
                  {isLoading && <Button onClick={onCancel}>Cancel</Button>}
                  <Text size="1" align="center">
                    We will send you a one-time login link by email.
                  </Text>
                </form>
              </Flex>
            </Flex>
          </Accordion.Content>
        </Accordion.Item>
        <Accordion.Item value="private">
          <Accordion.Trigger>Private Server</Accordion.Trigger>
          <Accordion.Content>
            <Box>
              <Text size="2">
                <ul>
                  <li>
                    User your own Refact server (Enterprise or self-hosted).
                  </li>
                  <li>Fine-tune code completions to your codebase</li>
                  <li>Keep all code and data under your control.</li>
                </ul>
              </Text>
            </Box>
            <Separator size="4" my="4" />
            <Flex asChild direction="column" gap="3" mb="2">
              {/** TODO: handle these changes */}
              <form
                onSubmit={(event) => {
                  const formData = new FormData(event.currentTarget);
                  const endpoint = formData.get("endpoint");
                  const apiKey = formData.get("api-key");
                  if (
                    apiKey &&
                    typeof apiKey === "string" &&
                    endpoint &&
                    typeof endpoint === "string"
                  ) {
                    setupHost({
                      type: "enterprise",
                      apiKey,
                      endpointAddress: endpoint,
                    });
                  } else if (endpoint && typeof endpoint === "string") {
                    setupHost({ type: "self", endpointAddress: endpoint });
                  }
                  // handle setUpHost
                }}
              >
                <Box>
                  <Text as="label" htmlFor="endpoint">
                    Endpoint
                  </Text>
                  <TextField.Root
                    type="url"
                    name="endpoint"
                    placeholder="http://x.x.x.x:8008/"
                    required
                  />
                </Box>

                <Box>
                  <Text as="label" htmlFor="api-key">
                    API Key (optional)
                  </Text>
                  <TextField.Root name="api-key" placeholder="your api key" />
                </Box>

                <Flex justify="end">
                  <Button type="submit">Open in IDE</Button>
                </Flex>
              </form>
            </Flex>
          </Accordion.Content>
        </Accordion.Item>
        <Accordion.Item value="byok">
          <Accordion.Trigger>Bring your own key</Accordion.Trigger>
          <Accordion.Content>
            <Box>
              <Text size="2">
                <ul>
                  <li>Connect to any OpenAI or Huggingface style server.</li>
                  <li>
                    Separate endpoints and keys for chat, completion, and
                    embedding.
                  </li>
                </ul>
              </Text>
            </Box>
            <Separator size="4" my="4" />
            <Flex gap="3" justify="end" mb="2">
              <Button
                onClick={() => {
                  setupHost({ type: "bring-your-own-key" });
                }}
              >
                Open in IDE
              </Button>
            </Flex>
          </Accordion.Content>
        </Accordion.Item>
      </Accordion.Root>
    </Container>
  );
};
