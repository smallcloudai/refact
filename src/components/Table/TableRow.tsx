import React from "react";

export const TableRow: React.FC<{ children: React.ReactNode; key?: number }> = (
  props,
) => <tr {...props} />;
