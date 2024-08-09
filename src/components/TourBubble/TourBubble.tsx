import { Flex, Link } from "@radix-ui/themes";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import { RootState } from "../../app/store";
import { close, next } from "../../features/Tour";
import { useWindowDimensions } from "../../hooks/useWindowDimensions";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { MutableRefObject, useEffect, useState } from "react";

export type TourBubbleProps = {
  text: string;
  step: number;
  down: boolean;
  target: MutableRefObject<HTMLElement | null>;
};

export function TourBubble({ text, step, target, down }: TourBubbleProps) {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);
  const { width: windowWidth } = useWindowDimensions();

  const isBubbleOpen = state.type === "in_progress" && state.step === step;
  const pos = target.current?.getBoundingClientRect();

  // TODO: find a better way of doing this
  // This code is there to force a rerender if target is null
  const [i, setI] = useState(0);
  useEffect(() => {
    setTimeout(() => {
      if (pos === undefined) setI(i + 1);
    }, 0);
  }, []);

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
          <TourBox>
            <TourTitle title={text} />
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
          </TourBox>
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
