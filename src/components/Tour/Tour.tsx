import React from "react";
import { TourBubble } from ".";
import { next, useTourRefs } from "../../features/Tour";
import { TourEnd } from "./TourEnd";
import { PageAction } from "../../hooks/usePages";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import { RootState } from "../../app/store";

type TourProps = {
  page: string;
  navigate: React.Dispatch<PageAction>;
};

export const Tour = ({ page, navigate }: TourProps) => {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);
  const refs = useTourRefs();

  if (state.type === "in_progress" && state.step === 2 && page === "chat") {
    dispatch(next());
  }

  if (state.type === "in_progress" && state.step === 7 && page === "history") {
    dispatch(next());
  }

  const chatWidth = "calc(100% - 20px)";

  return (
    <>
      <TourBubble
        text="When you write code, Refact already knows what comes next."
        step={1}
        target={refs.newChat}
        down={true}
        isPointing={false}
        onPage={"history"}
        page={page}
      />
      <TourBubble
        text="Ask questions in the Chat, it already knows your codebase."
        step={2}
        target={refs.newChat}
        down={true}
        onPage={"history"}
        page={page}
        onNext={() => navigate({ type: "push", page: { name: "chat" } })}
      />
      <TourBubble
        text={
          "The model autonomously calls functions to gather the best context for answers. When you’re not asking about your codebase, you can turn it off. "
        }
        step={3}
        down={false}
        target={refs.useTools}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
      />
      <TourBubble
        text={
          "Choose the latest LLMs for Chat. With the Pro plan, you get access to all the models."
        }
        step={4}
        down={false}
        target={refs.useModel}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
      />
      <TourBubble
        text={
          "There are @-commands to fill the context manually, for details type @help."
        }
        step={5}
        down={false}
        target={refs.chat}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
      />
      <TourBubble
        text={"Use 'New Chat' to switch topics and create a new thread."}
        step={6}
        down={true}
        target={refs.newChatInside}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
      />
      <TourBubble
        text={"Click ‘Back’ to see your chat history and continue discussion."}
        step={7}
        down={true}
        target={refs.back}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
        onNext={() => navigate({ type: "pop_back_to", page: "history" })}
      />
      <TourBubble
        text={"Click here to discover more."}
        step={8}
        down={false}
        target={refs.more}
        onPage={"history"}
        page={page}
      />
      <TourEnd step={9} />
    </>
  );
};
