import React, { useCallback, useMemo } from "react";
import { Flex, Skeleton, Text, Box } from "@radix-ui/themes";
import { Select } from "../Select";
import type { SystemPrompts } from "../../services/refact";
import {
  useAppDispatch,
  useAppSelector,
  useGetPromptsQuery,
} from "../../hooks";
import { getSelectedSystemPrompt } from "../../features/Chat/Thread/selectors";
import { setSystemPrompt } from "../../features/Chat/Thread/actions";

export const PromptSelect: React.FC = () => {
  const dispatch = useAppDispatch();
  const promptsRequest = useGetPromptsQuery();
  const selectedSystemPrompt = useAppSelector(getSelectedSystemPrompt);
  const onSetSelectedSystemPrompt = useCallback(
    (prompt: SystemPrompts) => dispatch(setSystemPrompt(prompt)),
    [dispatch],
  );

  const handleChange = useCallback(
    (key: string) => {
      if (!promptsRequest.data) return;
      if (!(key in promptsRequest.data)) return;
      const promptValue = promptsRequest.data[key];
      const prompt = { [key]: promptValue };
      onSetSelectedSystemPrompt(prompt);
    },
    [onSetSelectedSystemPrompt, promptsRequest.data],
  );

  const val = useMemo(
    () => Object.keys(selectedSystemPrompt)[0] ?? "default",
    [selectedSystemPrompt],
  );

  const options = useMemo(() => {
    return Object.entries(promptsRequest.data ?? {}).map(([key, value]) => {
      return {
        value: key,
        title: value.description || value.text,
      };
    });
  }, [promptsRequest.data]);

  return (
    <Flex
      gap="2"
      align="center"
      wrap="wrap"
      flexGrow="1"
      flexShrink="0"
      width="100%"
    >
      <Text size="2" wrap="nowrap">
        System Prompt:
      </Text>
      <Skeleton loading={promptsRequest.isLoading || promptsRequest.isFetching}>
        <Box flexGrow="1" flexShrink="0">
          <Select
            name="system prompt"
            disabled={promptsRequest.isLoading}
            onChange={handleChange}
            value={val}
            options={options}
          />
        </Box>
      </Skeleton>
    </Flex>
  );
};
