import { TourBubble } from ".";
import { useTourRefs } from "../../features/Tour";

type TourProps = {
  page: string;
};

export const Tour = ({ page }: TourProps) => {
  const refs = useTourRefs();

  const chatWidth = "calc(100% - 80px)";

  return (
    <>
      <TourBubble
        text="When you write code, Refact already knows what comes next."
        step={1}
        target={refs.newChat}
        down={true}
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
        text={"Click ‘Open in Tab’ to view the chat in full screen."}
        step={6}
        down={true}
        target={refs.openInNewTab}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
      />
      <TourBubble
        text={"Use 'New Chat' to switch topics and create a new thread."}
        step={7}
        down={true}
        target={refs.newChatInside}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
      />
      <TourBubble
        text={"Click ‘Back’ to see your chat history and continue discussion."}
        step={8}
        down={true}
        target={refs.back}
        containerWidth={chatWidth}
        onPage={"chat"}
        page={page}
      />
      <TourBubble
        text={"Click here to discover more."}
        step={9}
        down={false}
        target={refs.more}
        onPage={"history"}
        page={page}
      />
    </>
  );
};
