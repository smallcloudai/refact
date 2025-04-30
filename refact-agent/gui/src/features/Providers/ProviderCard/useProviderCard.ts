import { type MouseEventHandler, useCallback } from "react";
import { ProviderCardProps } from "./ProviderCard";
import { useUpdateProvider } from "../useUpdateProvider";

export function useProviderCard({
  provider,
  setCurrentProvider,
}: {
  provider: ProviderCardProps["provider"];
  setCurrentProvider: ProviderCardProps["setCurrentProvider"];
}) {
  const { updateProviderEnabledState, isUpdatingEnabledState } =
    useUpdateProvider({ provider });

  const handleClickOnProvider = useCallback(() => {
    if (isUpdatingEnabledState) return;

    setCurrentProvider(provider);
  }, [setCurrentProvider, provider, isUpdatingEnabledState]);

  const handleSwitchClick: MouseEventHandler<HTMLDivElement> = (event) => {
    if (isUpdatingEnabledState) return;

    event.stopPropagation();
    void updateProviderEnabledState();
  };

  return {
    handleClickOnProvider,
    handleSwitchClick,
  };
}
