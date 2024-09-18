import React from "react";
import { Flex, Button } from "@radix-ui/themes";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { ScrollArea } from "../../components/ScrollArea";
// import type { StatisticState } from "../../hooks";
import { StatisticView } from "../../components/StatisticView/StatisticView";
import { PageWrapper } from "../../components/PageWrapper";
import { useGetStatisticDataQuery } from "../../hooks";
import type { Config } from "../Config/configSlice";

export type StatisticsProps = {
  onCloseStatistic?: () => void;
  backFromStatistic: () => void;
  host: Config["host"];
  tabbed: Config["tabbed"];
  // state: StatisticState;
};

export const Statistics: React.FC<StatisticsProps> = ({
  onCloseStatistic,
  backFromStatistic,
  host,
  tabbed,
}) => {
  const LeftRightPadding =
    host === "web"
      ? { initial: "2", xl: "9" }
      : {
          initial: "2",
          xs: "2",
          sm: "4",
          md: "8",
          lg: "8",
          xl: "9",
        };

  const state = useGetStatisticDataQuery();

  return (
    <PageWrapper host={host}>
      {host === "vscode" && !tabbed ? (
        <Flex gap="2" pb="3">
          <Button variant="surface" onClick={backFromStatistic}>
            <ArrowLeftIcon width="16" height="16" />
            Back
          </Button>
        </Flex>
      ) : (
        <Button mr="auto" variant="outline" onClick={onCloseStatistic} mb="4">
          Back
        </Button>
      )}
      <ScrollArea scrollbars="vertical">
        <Flex
          direction="column"
          justify="between"
          flexGrow="1"
          mr={LeftRightPadding}
          style={{
            width: "inherit",
          }}
        >
          <StatisticView
            statisticData={state.data}
            isLoading={state.isLoading}
            error={state.error ? "Error fetching statiscs" : ""}
          />
        </Flex>
      </ScrollArea>
    </PageWrapper>
  );
};
