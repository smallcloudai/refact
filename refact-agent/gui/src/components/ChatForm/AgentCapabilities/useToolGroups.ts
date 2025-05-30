import { useCallback, useEffect, useMemo, useState } from "react";
import { useAppDispatch } from "../../../hooks";
import { useUpdateToolGroupsMutation } from "../../../hooks/useUpdateToolGroupsMutation";
import {
  Tool,
  ToolGroup,
  ToolGroupUpdate,
  toolsApi,
  ToolSpec,
} from "../../../services/refact";

export function useToolGroups() {
  const dispatch = useAppDispatch();
  const { mutationTrigger: updateToolGroups } = useUpdateToolGroupsMutation();

  const [selectedToolGroup, setSelectedToolGroup] = useState<ToolGroup | null>(
    null,
  );
  const [selectedToolGroupTools, setSelectedToolGroupTools] = useState<
    Tool[] | null
  >(null);

  const selectToolGroup = useCallback(
    (group: ToolGroup | null) => {
      setSelectedToolGroup(group);
    },
    [setSelectedToolGroup],
  );

  const someToolsEnabled = useMemo(() => {
    if (!selectedToolGroupTools) return false;
    return selectedToolGroupTools.some((tool) => tool.enabled);
  }, [selectedToolGroupTools]);

  const handleUpdateToolGroups = useCallback(
    ({
      updatedTools,
      updatedGroup,
    }: {
      updatedTools: { enabled: boolean; spec: ToolSpec }[];
      updatedGroup: ToolGroup;
    }) => {
      const dataToSend: ToolGroupUpdate[] = updatedTools.map((tool) => ({
        enabled: tool.enabled,
        source: tool.spec.source,
        name: tool.spec.name,
      }));

      updateToolGroups(dataToSend)
        .then((result) => {
          if (result.data) {
            // TODO: reduce complexity
            // it means, individual tool update
            if (selectedToolGroupTools && updatedTools.length === 1) {
              setSelectedToolGroupTools((prev) => {
                const tool = updatedTools[0];
                return prev
                  ? prev.map((t) => {
                      if (t.spec.name === tool.spec.name) {
                        return { ...t, enabled: tool.enabled };
                      }
                      return t;
                    })
                  : selectedToolGroupTools;
              });
              return;
            }
            setSelectedToolGroup(updatedGroup);
          }
        })
        .catch((error: unknown) => {
          // eslint-disable-next-line no-console
          console.log(error);
        });
    },
    [updateToolGroups, setSelectedToolGroupTools, selectedToolGroupTools],
  );

  const toggleAllTools = useCallback(
    (toolGroup: ToolGroup) => {
      const updatedTools = toolGroup.tools.map((tool) => ({
        ...tool,
        enabled: someToolsEnabled ? false : true,
      }));

      const updatedGroup = { ...toolGroup, tools: updatedTools };

      handleUpdateToolGroups({
        updatedTools,
        updatedGroup,
      });
    },
    [handleUpdateToolGroups, someToolsEnabled],
  );

  const toggleTool = useCallback(
    ({
      tool,
      parentGroup,
      togglingTo,
    }: {
      tool: ToolGroup["tools"][number];
      parentGroup: ToolGroup;
      togglingTo: boolean;
    }) => {
      const updatedTools: Tool[] = [
        {
          enabled: togglingTo,
          spec: tool.spec,
        },
      ];

      const updatedGroup = {
        ...parentGroup,
        tools: parentGroup.tools.map((t) => {
          if (t.spec.name === tool.spec.name) {
            return { ...tool };
          }

          return { ...t };
        }),
      };

      handleUpdateToolGroups({
        updatedTools,
        updatedGroup,
      });
    },
    [handleUpdateToolGroups],
  );

  const resetSelection = useCallback(() => {
    dispatch(toolsApi.util.invalidateTags(["TOOL_GROUPS"]));
    setSelectedToolGroup(null);
  }, [dispatch]);

  useEffect(() => {
    if (selectedToolGroup) {
      setSelectedToolGroupTools(selectedToolGroup.tools);
    }
  }, [selectedToolGroup]);

  return {
    toggleTool,
    toggleAllTools,
    resetSelection,
    selectToolGroup,
    selectedToolGroup,
    selectedToolGroupTools,
    someToolsEnabled,
  };
}
