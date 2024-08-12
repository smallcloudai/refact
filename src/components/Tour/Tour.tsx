import { TourBubble } from ".";
import { useTourRefs } from "../../features/Tour";

export const Tour = () => {
  const refs = useTourRefs();

  return (
    <>
      <TourBubble
        text="When you write code, Refact already knows what comes next."
        step={1}
        target={refs.newChat}
        down={true}
      />
      <TourBubble
        text="Ask questions in the Chat, it already knows your codebase."
        step={2}
        target={refs.newChat}
        down={true}
      />
      <TourBubble
        text={
          "The model autonomously calls functions to gather the best context for answers. When you’re not asking about your codebase, you can turn it off. "
        }
        step={3}
        down={false}
        target={refs.useTools}
        containerWidth="calc(100% - 40px)"
      />
    </>
  );
};
