import { Dialog, Flex, Text, Button } from "@radix-ui/themes";
import { useCheckpoints } from "../../hooks/useCheckpoints";
import { useEventsBusForIDE } from "../../hooks/useEventBusForIDE";
import { TruncateLeft } from "../../components/Text";
import { Link } from "../../components/Link";
import { ScrollArea } from "../../components/ScrollArea";

import styles from "./Checkpoints.module.css";
import { formatDateOrTimeBasedOnToday } from "../../utils/formatDateToHumanReadable";
import { formatPathName } from "../../utils/formatPathName";
import { CheckpointsStatusIndicator } from "./CheckpointsStatusIndicator";
import { ErrorCallout } from "../../components/Callout";

export const Checkpoints = () => {
  const { openFile } = useEventsBusForIDE();
  const {
    shouldCheckpointsPopupBeShown,
    handleFix,
    handleUndo,
    reverted_to,
    isRestoring,
    allChangedFiles,
    wereFilesChanged,
    errorLog,
  } = useCheckpoints();

  const clientTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const formattedDate = formatDateOrTimeBasedOnToday(
    reverted_to,
    clientTimezone,
  );

  const checkpointsTitle = `${
    wereFilesChanged ? "Files changed" : "No files were changed"
  } from checkpoint at ${formattedDate}`;

  return (
    <Dialog.Root
      open={shouldCheckpointsPopupBeShown}
      onOpenChange={(state) => {
        if (!state) {
          handleUndo();
        } else {
          void handleFix();
        }
      }}
    >
      <Dialog.Content className={styles.CheckpointsDialog}>
        <Dialog.Description size="1" color="gray">
          Restores chat and your project&apos;s files back to a snapshot taken
          at this point
        </Dialog.Description>
        <Dialog.Title as="h2" size="3" mt="4" mb="3">
          {errorLog.length >= 1
            ? "Oops... Something went wrong"
            : checkpointsTitle}
        </Dialog.Title>
        <ScrollArea scrollbars="vertical" style={{ maxHeight: "300px" }}>
          <Flex direction="column" gap="2">
            {wereFilesChanged &&
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
                    <Flex align="center" gap="2" width="100%">
                      <TruncateLeft size="2" style={{ maxWidth: "50%" }}>
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

                      <CheckpointsStatusIndicator status={file.status} />
                    </Flex>
                  </Flex>
                );
              })}
          </Flex>
        </ScrollArea>
        {errorLog.length > 0 && (
          <ErrorCallout mx="0" preventRetry>
            {errorLog.join("\n")}
          </ErrorCallout>
        )}
        <Flex
          gap="3"
          mt={wereFilesChanged ? "4" : "2"}
          justify="between"
          wrap="wrap"
        >
          <Flex gap="3" wrap="wrap" justify="start">
            <Button
              type="button"
              variant="soft"
              color="gray"
              onClick={handleUndo}
            >
              Cancel
            </Button>
            <Button
              loading={isRestoring}
              disabled={errorLog.length > 0}
              onClick={() => void handleFix()}
              title={
                isRestoring
                  ? "Rolling back..."
                  : errorLog.length > 0
                    ? "There are some errors, you cannot roll back to this checkpoint"
                    : "Roll back to checkpoint"
              }
            >
              Roll back to checkpoint
            </Button>
          </Flex>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};
