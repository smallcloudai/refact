import React, {
  createContext,
  useContext,
  useState,
  ReactNode,
  useMemo,
  useCallback,
} from "react";

type ProviderUpdateState = {
  updatingProviders: Record<string, boolean>;
  setProviderUpdating: (providerName: string, isUpdating: boolean) => void;
};

const ProviderUpdateContext = createContext<ProviderUpdateState | undefined>(
  undefined,
);

export const ProviderUpdateProvider: React.FC<{ children: ReactNode }> = ({
  children,
}) => {
  const [updatingProviders, setUpdatingProviders] = useState<
    Record<string, boolean>
  >({});

  const setProviderUpdating = useCallback(
    (providerName: string, isUpdating: boolean) => {
      setUpdatingProviders((prev) => ({
        ...prev,
        [providerName]: isUpdating,
      }));
    },
    [],
  );

  const value = useMemo(
    () => ({ updatingProviders, setProviderUpdating }),
    [updatingProviders, setProviderUpdating],
  );

  return (
    <ProviderUpdateContext.Provider value={value}>
      {children}
    </ProviderUpdateContext.Provider>
  );
};

// eslint-disable-next-line react-refresh/only-export-components
export const useProviderUpdateContext = (): ProviderUpdateState => {
  const context = useContext(ProviderUpdateContext);
  if (context === undefined) {
    throw new Error(
      "useProviderUpdateContext must be used within a ProviderUpdateProvider",
    );
  }
  return context;
};
