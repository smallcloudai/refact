import React from "react";
import { useGetMCPLogs } from "./useGetMCPLogs";

type MCPLogsProps = {
  integrationPath: string;
};

export const MCPLogs: React.FC<MCPLogsProps> = ({ integrationPath }) => {
  const { data, isLoading } = useGetMCPLogs(integrationPath);

  if (!data) {
    if (isLoading) {
      return <div>Loading...</div>;
    }
    return <div>No data</div>;
  }

  return <div>{JSON.stringify(data.logs, null, 2)}</div>;
};
