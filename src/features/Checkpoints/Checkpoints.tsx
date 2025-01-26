import { Dialog, Flex, Text, Button } from "@radix-ui/themes";
import { useCheckpoints } from "../../hooks/useCheckpoints";
import { useEventsBusForIDE } from "../../hooks";
import { TruncateLeft } from "../../components/Text";
import { Link } from "../../components/Link";
import { ScrollArea } from "../../components/ScrollArea";

import styles from "./Checkpoints.module.css";
import { formatDateToHumanReadable } from "../../utils/formatDateToHumanReadable";
import { formatPathName } from "../../utils/formatPathName";
import { CheckpointsStatusIndicator } from "./CheckpointsStatusIndicator";

export const Checkpoints = () => {
  const { openFile } = useEventsBusForIDE();
  const {
    shouldCheckpointsPopupBeShown,
    handleFix,
    handleUndo,
    handleShouldMockBeUsedChange,
    reverted_to,
    isLoading,
    allChangedFiles,
    wereFilesChanged,
    shouldMockBeUsed,
  } = useCheckpoints();

  const clientTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const formattedDate = formatDateToHumanReadable(reverted_to, clientTimezone);

  return (
    <Dialog.Root open={shouldCheckpointsPopupBeShown}>
      <Dialog.Content className={styles.CheckpointsDialog}>
        <Text size="1" color="gray" className={styles.CheckpointsRevertedDate}>
          Reverted to date: {formattedDate}
        </Text>
        <Dialog.Title as="h3" size="3" mb="3">
          Reverted files from checkpoints
        </Dialog.Title>
        <ScrollArea scrollbars="vertical" style={{ maxHeight: "300px" }}>
          <Flex direction="column" gap="2">
            {allChangedFiles.length > 0 ? (
              allChangedFiles.map((file, index) => {
                const formattedWorkspaceFolder = formatPathName(
                  file.workspace_folder,
                );
                return (
                  <Flex
                    key={`${file.absolute_path}-${index}`}
                    gap="2"
                    py="2"
                    px="2"
                    justify="between"
                    align="center"
                    style={{
                      backgroundColor: "var(--gray-3)",
                      borderRadius: "var(--radius-3)",
                    }}
                  >
                    <Flex align="center" gap="2">
                      <TruncateLeft size="2">
                        <Link
                          title="Open file"
                          onClick={(event) => {
                            event.preventDefault();
                            openFile({ file_name: file.absolute_path });
                          }}
                          style={{
                            textDecoration:
                              file.status === "DELETED"
                                ? "line-through"
                                : undefined,
                          }}
                        >
                          {formatPathName(file.absolute_path)}
                        </Link>
                      </TruncateLeft>
                      <Text size="2" color="gray" style={{ opacity: 0.65 }}>
                        {formattedWorkspaceFolder}
                      </Text>
                    </Flex>
                    <CheckpointsStatusIndicator status={file.status} />
                  </Flex>
                );
              })
            ) : (
              <Flex>
                <Text as="p" size="2">
                  No files changed
                </Text>
              </Flex>
            )}
          </Flex>
        </ScrollArea>

        <Flex gap="3" mt="4" justify="between">
          <Button
            variant="soft"
            color="purple"
            onClick={handleShouldMockBeUsedChange}
          >
            {shouldMockBeUsed ? "Use real data" : "Use mock data"}
          </Button>
          <Flex gap="3">
            {wereFilesChanged && (
              <Button
                variant="soft"
                color="gray"
                loading={isLoading}
                onClick={() => void handleUndo()}
              >
                Undo
              </Button>
            )}
            <Button onClick={handleFix}>
              {wereFilesChanged ? "Fix" : "Close"}
            </Button>
          </Flex>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};
