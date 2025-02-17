import { useContext } from "react";
import { AbortControllerContext } from "../contexts/AbortControllers";

export const useAbortControllers = () => {
  const context = useContext(AbortControllerContext);
  if (context === null) {
    throw new Error(
      "useAbortControllers must be used within a AbortControllerProvider",
    );
  }
  return context;
};
