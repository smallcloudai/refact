import { Flex, Link, useThemeContext } from "@radix-ui/themes";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import { RootState } from "../../app/store";
import { close, next } from "../../features/Tour";
import { useWindowDimensions } from "../../hooks/useWindowDimensions";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { ReactNode, useEffect, useState } from "react";

export type TourBubbleProps = {
  text: string;
  step: number;
  down: boolean;
  isPointing?: boolean;
  target: HTMLElement | null;
  containerWidth?: string;
  onPage: string;
  page: string;
  children?: ReactNode;
  onNext?: () => void;
};

export function TourBubble({
  text,
  step,
  target,
  down,
  containerWidth,
  onPage,
  page,
  isPointing,
  children,
  onNext,
}: TourBubbleProps) {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);
  const { width: windowWidth, height: windowHeight } = useWindowDimensions();
  const [pos, setPos] = useState<DOMRect | undefined>(undefined);
  const appearance = useThemeContext().appearance;

  const isBubbleOpen = state.type === "in_progress" && state.step === step;

  if (isPointing === undefined) {
    isPointing = true;
  }

  // TODO: find a better way of doing this
  // This code is there to force a rerender if target is null
  useEffect(() => {
    if (target === null || page !== onPage) {
      setPos(undefined);
    } else {
      const newPos = target.getBoundingClientRect();
      if (
        pos?.left !== newPos.left ||
        pos.right !== newPos.right ||
        pos.top !== newPos.top ||
        pos.bottom !== newPos.bottom
      ) {
        setPos(newPos);
      }
    }
  }, [page, onPage, target, pos, setPos, windowWidth, windowHeight]);

  if (pos === undefined) {
    return <></>;
  }

  const centX = (pos.left + pos.right) / 2 - windowWidth / 2;
  const arrowColor = appearance == "dark" ? "white" : "black";

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
            width: containerWidth ?? "min(calc(100% - 20px), 540px)",
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
                borderBottom: `15px solid ${arrowColor}`,
                alignSelf: "center",
                position: "relative",
                opacity: isPointing ? 1 : 0,
                left: centX,
              }}
            />
          )}
          <TourBox>
            <TourTitle title={text} />
            {children}
            <Link
              style={{
                cursor: "pointer",
                position: "absolute",
                right: "8px",
                top: "1px",
                color: appearance == "dark" ? "black" : "white",
              }}
              onClick={() => {
                dispatch(close());
              }}
            >
              ×
            </Link>
            <Link
              style={{
                cursor: "pointer",
                position: "absolute",
                right: "10px",
                bottom: "10px",
                textTransform: "uppercase",
                fontSize: "12px",
                fontWeight: "bold",
                color: appearance == "dark" ? "#004069" : "#54a1ff",
              }}
              onClick={() => {
                dispatch(next());
                if (onNext) onNext();
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
                borderTop: `15px solid ${arrowColor}`,
                alignSelf: "center",
                position: "relative",
                opacity: isPointing ? 1 : 0,
                left: centX,
              }}
            />
          )}
        </Flex>
      </Flex>
    )
  );
}
