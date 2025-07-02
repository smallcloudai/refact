import React, { useCallback, useEffect } from "react";
import { TourBubble } from "./TourBubble";
import { next, useTourRefs } from "../../features/Tour";
import { useAppSelector, useAppDispatch } from "../../hooks";
import { RootState } from "../../app/store";
import { push } from "../../features/Pages/pagesSlice";
import completionGif from "../../../public/completion.gif";
import commandsGif from "../../../public/commands.gif";
import agentGif from "../../../public/agent.gif";

export type TourProps = {
  page: string;
};

export const Tour: React.FC<TourProps> = ({ page }) => {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);
  const refs = useTourRefs();

  const openChat = useCallback(() => {
    dispatch(push({ name: "chat" }));
  }, [dispatch]);

  // const openHistory = useCallback(() => {
  //   dispatch(push({ name: "history" }));
  // }, [dispatch]);

  const step = state.type === "in_progress" ? state.step : 0;

  useEffect(() => {
    if (state.type === "in_progress" && step === 1 && page === "chat") {
      dispatch(next());
    }

    if (state.type === "in_progress" && step === 4 && page === "history") {
      dispatch(next());
    }

    if (state.type === "in_progress" && step === 6 && page === "history") {
      dispatch(push({ name: "tour end" }));
    }

    if (state.type === "finished" && page === "tour end") {
      dispatch(push({ name: "history" }));
    }
  }, [state.type, step, page, dispatch]);

  // const chatWidth = "calc(100% - 20px)";

  // TODO: Did the Popover or HoverCard components not work for this?
  return (
    <>
      <TourBubble
        title="Agent can accomplish tasks end to end"
        text={`Write anything you want to do and Refact.ai Agent will\n- inspect your files\n- write the code\n- run shell commands if needed\n- apply the code in your files\n- open browser to check if changes are correct in case of UI`}
        step={1}
        target={refs.newChat}
        down={true}
        isPointing={false}
        onPage={"history"}
        onNext={openChat}
        page={page}
        deltaY={-40}
      >
        <img
          style={{ marginTop: "10px", marginBottom: "30px" }}
          src={agentGif}
        />
      </TourBubble>
      {/* <TourBubble
        title="Integrations"
        text={
          "In order for agent to work properly you need to set up integrations. Just click on this button and follow the instructions."
        }
        step={2}
        down={false}
        target={refs.setupIntegrations}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
        bubbleContainerStyles={{
          alignSelf: "flex-end",
        }}
      /> */}
      <TourBubble
        title="Chat modes / models"
        text={`Our chat allows you to\n- use images to give more context\n- specify context use @commands, write @help to view`}
        step={3}
        target={refs.chat}
        onPage={"chat"}
        page={page}
        down={false}
      >
        <img
          style={{
            marginTop: "10px",
            marginBottom: "30px",
          }}
          src={commandsGif}
        />
      </TourBubble>
      {/* <TourBubble
        title="Difference in Quick / Explore / Agent"
        text={`Switch inside of the chat let you to choose the chat mode:\n- Quick for immediate answers, no tools and context access\n- Explore for ideating and learning, chat can access the context but all changes are performed manually\n- Agent for tasks where you expect chat to make changes autonomously`}
        step={4}
        down={false}
        target={refs.useTools}
        containerWidth={chatWidth}
        onPage={"chat"}
        onNext={openHistory}
        page={page}
        bubbleContainerStyles={{
          maxWidth: 550,
          alignSelf: "start",
        }}
      /> */}
      <TourBubble
        title="Code completion"
        text={`- we use context from your entire repository\n- you can adjust the number of output tokens in Plugin settings`}
        step={5}
        target={refs.newChat}
        down={true}
        isPointing={false}
        onPage={"history"}
        page={page}
        deltaY={-40}
      >
        <img
          style={{
            marginTop: "10px",
            marginBottom: "30px",
          }}
          src={completionGif}
        />
      </TourBubble>
    </>
  );
};
