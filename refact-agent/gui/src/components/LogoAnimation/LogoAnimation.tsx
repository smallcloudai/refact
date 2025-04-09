import React, { useEffect, useMemo } from "react";
import Lottie, {
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
};

function sizeToCssVariable(size: AnimationSize) {
  const sizeString = `var(--font-size-${size}, calc(16px * var(--scaling, 1))`;
  return { width: sizeString, height: sizeString };
}

export const LogoAnimation: React.FC<LogoAnimationProps> = ({
  size = defaultSize,
  ...props
}) => {
  const options: LottieOptions = useMemo(() => {
    const sizeProps = sizeToCssVariable(size);
    const styleProps = { ...sizeProps, ...props.style };
    return {
      ...props,
      style: styleProps,
      animationData: logoAnimationData,
    };
  }, [props, size]);

  const { View, getDuration, goToAndStop } = useLottie(options);
  useEffect(() => {
    const duration = getDuration();
    if (!props.loop && duration) {
      goToAndStop(duration);
    }
  }, [getDuration, goToAndStop, props.loop]);

  return View;
};
