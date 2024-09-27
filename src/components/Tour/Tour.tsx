import React, { useCallback, useEffect } from "react";
import { TourBubble } from "./TourBubble";
import { next, useTourRefs } from "../../features/Tour";
import { useAppSelector, useAppDispatch } from "../../hooks";
import { RootState } from "../../app/store";
import { push } from "../../features/Pages/pagesSlice";
import completionGif from "../../../public/completion.gif";
import { newChatAction } from "../../events";

export type TourProps = {
  page: string;
};

export const Tour: React.FC<TourProps> = ({ page }) => {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);
  const refs = useTourRefs();

  const openChat = useCallback(() => {
    dispatch(newChatAction());
    dispatch(push({ name: "chat" }));
  }, [dispatch]);

  const openHistory = useCallback(() => {
    dispatch(push({ name: "history" }));
  }, [dispatch]);

  const step = state.type === "in_progress" ? state.step : 0;

  useEffect(() => {
    if (state.type === "in_progress" && step === 2 && page === "chat") {
      dispatch(next());
    }

    if (state.type === "in_progress" && step === 6 && page === "history") {
      dispatch(next());
    }

    if (state.type === "in_progress" && step === 8 && page === "history") {
      dispatch(push({ name: "tour end" }));
    }

    if (state.type === "finished" && page === "tour end") {
      dispatch(push({ name: "history" }));
    }
  }, [state.type, step, page, dispatch]);

  const chatWidth = "calc(100% - 20px)";

  // TODO: Did the Popover or HoverCard components not work for this?
  return (
    <>
      <TourBubble
        text="When you write code, Refact can predict what comes next. Accept code suggestions with Tab. Refact uses RAG, not just the current file for code completion, check out how it works in Fill-in-the-middle Context in the menu."
        step={1}
        target={refs.newChat}
        down={true}
        isPointing={false}
        onPage={"history"}
        page={page}
        deltaY={-40}
      >
        <img
          style={{ marginTop: "10px", marginBottom: "30px" }}
          src={completionGif}
        />
      </TourBubble>
      <TourBubble
        text="Open a new chat using this button. Refact can stream several chats at the same time, this is most useful for agent functions that can run a long time. Chat that have unread responses will be marked with a small circle."
        step={2}
        target={refs.newChat}
        down={true}
        onPage={"history"}
        page={page}
        onNext={openChat}
        containerWidth={chatWidth}
        bubbleContainerStyles={{
          alignSelf: "flex-end",
        }}
      />
      <TourBubble
        text={
          "The Big Switch allows you to choose between immediate answers (Quick), automatic exploration tools that the model can use to collect the necessary context from your project (Explore), or slow agentic functions that can convert a lot of GPU cycles to useful work (Agent)."
        }
        step={3}
        down={false}
        target={refs.useTools}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
        bubbleContainerStyles={{
          alignSelf: "flex-start",
        }}
      />
      <TourBubble
        text={
          "Choose the language model you like more, for example Anthropic-3-5-sonnet clearly works better for Rust. With the Refact PRO plan, you will see more models in the list: the largest models from OpenAI, Anthropic for a fixed price. But FREE is great too, well, it's free!"
        }
        step={4}
        down={false}
        target={refs.useModel}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
        bubbleContainerStyles={{
          alignSelf: "flex-start",
        }}
      />
      <TourBubble
        text={
          "Sometimes you want to tell the model exactly what to take in as context, and not rely on automatic context collection. There are @-commands for you to do just that, for details type @help."
        }
        step={5}
        down={false}
        target={refs.chat}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
        bubbleContainerStyles={{
          maxWidth: 550,
          alignSelf: "center",
        }}
      />
      <TourBubble
        text={
          "Here under Home button you will see your chat history and some agentic functions soon."
        }
        step={6}
        down={true}
        target={refs.back}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
        onNext={openHistory}
        bubbleContainerStyles={{
          alignSelf: "flex-start",
        }}
      />
      <TourBubble
        text={
          "Click here for settings, keyboard shortcuts, customization, integrations, etc. There's a link to discord, too, for you to complain something isn't working or to happily report that it is!"
        }
        step={7}
        down={true}
        containerWidth={chatWidth}
        target={refs.more}
        onPage={"history"}
        page={page}
        bubbleContainerStyles={{
          alignSelf: "flex-end",
        }}
      />
    </>
  );
};
