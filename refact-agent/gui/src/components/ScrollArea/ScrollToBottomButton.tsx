import { ArrowDownIcon } from "@radix-ui/react-icons";
import { IconButton } from "@radix-ui/themes";

type ScrollToBottomButtonProps = {
  onClick: () => void;
};

export const ScrollToBottomButton = ({
  onClick,
}: ScrollToBottomButtonProps) => {
  return (
    <IconButton
      title="Follow stream"
      style={{
        position: "absolute",
        width: 35,
        height: 35,
        bottom: 15,
        right: 15,
        zIndex: 1,
      }}
      onClick={onClick}
    >
      <ArrowDownIcon width={21} height={21} />
    </IconButton>
  );
};
