import { useContext } from "react";
import { AbortControllerContext } from "../contexts/AbortControllers";

// TODO: can remove
export const useAbortControllers = () => {
  const context = useContext(AbortControllerContext);
  if (context === null) {
    throw new Error(
      "useAbortControllers must be used within a AbortControllerProvider",
    );
  }
  return context;
};
