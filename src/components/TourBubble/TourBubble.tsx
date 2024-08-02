import { Flex, Link, Text } from "@radix-ui/themes";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import { RootState } from "../../app/store";
import { close, next } from "../../features/Tour";

export type TourBubbleProps = {
  text: string;
  step: number;
};

export function TourBubble({ text, step }: TourBubbleProps) {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);

  const isBubbleOpen = state.type === "in_progress" && state.step === step;

  return (
    isBubbleOpen && (
      <Flex
        style={{
          position: "relative",
          height: 0,
          width: "100%",
          alignSelf: "center",
        }}
      >
        <Flex
          style={{
            position: "absolute",
            zIndex: 100,
            width: "calc(100% - 20px)",
            left: "10px",
            flexDirection: "column",
          }}
        >
          <Flex
            style={{
              width: 0,
              height: 0,
              borderLeft: "15px solid transparent",
              borderRight: "15px solid transparent",
              borderBottom: "15px solid white",
              alignSelf: "center",
            }}
          />
          <Flex
            style={{
              position: "relative",
              backgroundColor: "white",
              borderRadius: "5px",
              minHeight: "60px",
              alignItems: "center",
              padding: "7px",
            }}
          >
            <img src="favicon.png" width={28} height={28} />
            <Text style={{ color: "black", fontSize: 16, margin: 4 }}>
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
        </Flex>
      </Flex>
    )
  );
}
