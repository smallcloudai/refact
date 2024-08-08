import { Flex, Link, Text } from "@radix-ui/themes";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import { RootState } from "../../app/store";
import { close, next } from "../../features/Tour";
import { useWindowDimensions } from "../../hooks/useWindowDimensions";

export type TourBubbleProps = {
  text: string;
  step: number;
  down: boolean;
  target: HTMLElement | null;
};

export function TourBubble({ text, step, target, down }: TourBubbleProps) {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);
  const { width: windowWidth } = useWindowDimensions();

  const isBubbleOpen = state.type === "in_progress" && state.step === step;
  const pos = target?.getBoundingClientRect();

  if (pos === undefined) {
    return <></>;
  }

  const centX = (pos.left + pos.right) / 2 - windowWidth / 2;

  return (
    isBubbleOpen && (
      <Flex
        style={{
          flexDirection: "column",
          position: "fixed",
          height: 0,
          width: "100%",
          alignSelf: "center",
          top: down ? pos.bottom : pos.top,
          zIndex: 100,
        }}
      >
        <Flex
          style={{
            position: "absolute",
            width: "min(calc(100% - 20px), 540px)",
            flexDirection: "column",
            alignSelf: "center",
            bottom: down ? "auto" : 0,
            top: down ? 0 : "auto",
          }}
        >
          {down && (
            <Flex
              style={{
                width: 0,
                height: 0,
                borderLeft: "15px solid transparent",
                borderRight: "15px solid transparent",
                borderBottom: "15px solid white",
                alignSelf: "center",
                position: "relative",
                left: centX,
              }}
            />
          )}
          <Flex
            style={{
              position: "relative",
              backgroundColor: "white",
              borderRadius: "5px",
              minHeight: "60px",
              alignItems: "center",
              padding: "7px",
              alignSelf: "stretch",
            }}
          >
            <img src="favicon.png" width={28} height={28} />
            <Text
              style={{
                color: "black",
                fontSize: 16,
                margin: 4,
                paddingRight: 30,
              }}
            >
              {text}
            </Text>
            <Link
              style={{
                cursor: "pointer",
                position: "absolute",
                right: "8px",
                top: "1px",
                color: "black",
              }}
              onClick={() => {
                dispatch(close());
              }}
            >
              x
            </Link>
            <Link
              style={{
                cursor: "pointer",
                position: "absolute",
                right: "5px",
                bottom: "5px",
                color: "#3312a3",
              }}
              onClick={() => {
                dispatch(next());
              }}
            >
              next
            </Link>
          </Flex>
          {down || (
            <Flex
              style={{
                width: 0,
                height: 0,
                borderLeft: "15px solid transparent",
                borderRight: "15px solid transparent",
                borderTop: "15px solid white",
                alignSelf: "center",
                position: "relative",
                left: centX,
              }}
            />
          )}
        </Flex>
      </Flex>
    )
  );
}
