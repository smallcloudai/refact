import React, { useEffect, useMemo } from "react";
import {
  useLottie,
  type LottieOptions,
  type LottieComponentProps,
} from "lottie-react";
import logoAnimationData from "./animationData.json";
import { defaultSize, type AnimationSize } from "./types";

export type LogoAnimationProps = Omit<
  LottieComponentProps,
  "animationData" | "size"
> & {
  size?: AnimationSize;
  isWaiting: boolean;
  isStreaming: boolean;
};

function sizeToCssVariable(size: AnimationSize) {
  const sizeString = `var(--font-size-${size}, calc(16px * var(--scaling, 1))`;
  return { width: sizeString, height: sizeString };
}

const EYE_ANIMATION_FRAMES: [number, number] = [0, 70];
const CHAR_ANIMATION_FRAMES: [number, number] = [79, 213];
const SPIN_ANIMATION_FRAMES: [number, number] = [214, 254];
const HAPPY_EYES: [number, number] = [242, 250];

export const LogoAnimation: React.FC<LogoAnimationProps> = ({
  size = defaultSize,
  isWaiting,
  isStreaming,
  ...props
}) => {
  const options: LottieOptions = useMemo(() => {
    const sizeProps = sizeToCssVariable(size);
    const styleProps = { ...sizeProps, ...props.style };
    return {
      ...props,
      style: styleProps,
      animationData: logoAnimationData,
      loop: isStreaming || isWaiting,
    };
  }, [isStreaming, isWaiting, props, size]);

  const { View, playSegments } = useLottie(options);
  useEffect(() => {
    if (isWaiting && !isStreaming) {
      playSegments([EYE_ANIMATION_FRAMES, SPIN_ANIMATION_FRAMES], true);
    } else if (isStreaming) {
      playSegments(CHAR_ANIMATION_FRAMES, true);
    } else {
      playSegments(HAPPY_EYES, true);
    }
  }, [isStreaming, isWaiting, playSegments]);

  return View;
};
