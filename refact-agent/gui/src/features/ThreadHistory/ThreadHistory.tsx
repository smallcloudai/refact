import { FC, useCallback } from "react";
import { Config } from "../Config/configSlice";
import { Button, Flex } from "@radix-ui/themes";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { ChatRawJSON } from "../../components/ChatRawJSON";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { getChatById } from "../History/historySlice";
import { copyChatHistoryToClipboard } from "../../utils/copyChatHistoryToClipboard";
import { clearError, getErrorMessage, setError } from "../Errors/errorsSlice";
import {
  clearInformation,
  getInformationMessage,
  setInformation,
} from "../Errors/informationSlice";
import {
  ErrorCallout,
  InformationCallout,
} from "../../components/Callout/Callout";
import styles from "./ThreadHistory.module.css";

type ThreadHistoryProps = {
  onCloseThreadHistory: () => void;
  backFromThreadHistory: () => void;
  host: Config["host"];
  tabbed: Config["tabbed"];
  chatId: string;
};

export const ThreadHistory: FC<ThreadHistoryProps> = ({
  onCloseThreadHistory,
  backFromThreadHistory,
  host,
  tabbed,
  chatId,
}) => {
  const dispatch = useAppDispatch();

  const historyThread = useAppSelector((state) => getChatById(state, chatId), {
    devModeChecks: { stabilityCheck: "never" },
  });

  const historyThreadToPass = historyThread && {
    ...historyThread,
    model: historyThread.model || "gpt-4o-mini",
  };

  const error = useAppSelector(getErrorMessage);
  const information = useAppSelector(getInformationMessage);

  const onClearError = useCallback(() => dispatch(clearError()), [dispatch]);
  const onClearInformation = useCallback(
    () => dispatch(clearInformation()),
    [dispatch],
  );

  const handleCopyToClipboardJSON = useCallback(() => {
    if (!historyThread) {
      dispatch(setError("No history thread found"));
      return;
    }

    void copyChatHistoryToClipboard(historyThread).then(() => {
      dispatch(setInformation("Chat history copied to clipboard"));
    });
  }, [dispatch, historyThread]);

  const handleBackFromThreadHistory = useCallback(
    (customBackFunction: () => void) => {
      if (information) {
        onClearInformation();
      }
      if (error) {
        onClearError();
      }
      customBackFunction();
    },
    [information, error, onClearError, onClearInformation],
  );

  return (
    <>
      {host === "vscode" && !tabbed ? (
        <Flex gap="2" pb="3">
          <Button
            variant="surface"
            onClick={() => handleBackFromThreadHistory(backFromThreadHistory)}
          >
            <ArrowLeftIcon width="16" height="16" />
            Back
          </Button>
        </Flex>
      ) : (
        <Button
          mr="auto"
          variant="outline"
          onClick={() => handleBackFromThreadHistory(onCloseThreadHistory)}
          mb="4"
        >
          Back
        </Button>
      )}
      {historyThreadToPass && (
        <ChatRawJSON
          thread={historyThreadToPass}
          copyHandler={handleCopyToClipboardJSON}
        />
      )}
      {information && (
        <InformationCallout
          className={styles.calloutContainer}
          onClick={onClearInformation}
          timeout={3000}
        >
          {information}
        </InformationCallout>
      )}
      {error && (
        <ErrorCallout
          className={styles.calloutContainer}
          onClick={onClearError}
          timeout={3000}
        >
          {error}
        </ErrorCallout>
      )}
    </>
  );
};
