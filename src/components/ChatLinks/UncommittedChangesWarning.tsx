import React, { useEffect, useState } from "react";
import { useGetLinksFromLsp } from "../../hooks";
import { Markdown } from "../Markdown";
import { Callout, Flex, Box, Card } from "@radix-ui/themes";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import styles from "./UncommittedChangesWarning.module.css";

export const UncommittedChangesWarning: React.FC<{
  children?: React.ReactNode;
}> = ({ children }) => {
  const linksRequest = useGetLinksFromLsp();

  const [isOpened, setIsOpened] = useState<boolean>(
    !linksRequest.data?.uncommited_changes_warning,
  );

  useEffect(() => {
    if (linksRequest.data?.uncommited_changes_warning) {
      setIsOpened(true);
    } else {
      setIsOpened(false);
    }
  }, [linksRequest.data?.uncommited_changes_warning]);

  return (
    <Box>
      {isOpened && linksRequest.data?.uncommited_changes_warning && (
        <Card asChild>
          <Callout.Root
            color="amber"
            onClick={() => setIsOpened(false)}
            className={styles.changes_warning}
          >
            <Flex direction="row" align="center" gap="4" position="relative">
              <Callout.Icon>
                <InfoCircledIcon />
              </Callout.Icon>

              <Markdown wrap>
                {linksRequest.data.uncommited_changes_warning}
              </Markdown>
            </Flex>
          </Callout.Root>
        </Card>
      )}
      {children}
    </Box>
  );
};
