import {
  Button,
  DropdownMenu,
  Flex,
  IconButton,
  Spinner,
  TabNav,
  Text,
  TextField,
} from "@radix-ui/themes";
import { Dropdown, DropdownNavigationOptions } from "./Dropdown";
import {
  Cross1Icon,
  DotFilledIcon,
  DotsVerticalIcon,
  HomeIcon,
  PlusIcon,
} from "@radix-ui/react-icons";
import { newChatAction } from "../../events";
import { restart, useTourRefs } from "../../features/Tour";
import { popBackTo, push } from "../../features/Pages/pagesSlice";
import {
  ChangeEvent,
  KeyboardEvent,
  MouseEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  deleteChatById,
  updateChatTitleById,
} from "../../features/History/historySlice";
import {
  saveTitle,
  selectOpenThreadIds,
  selectAllThreads,
  closeThread,
  switchToThread,
  selectChatId,
  clearThreadPauseReasons,
  setThreadConfirmationStatus,
} from "../../features/Chat";
import { TruncateLeft } from "../Text";
import {
  useAppDispatch,
  useAppSelector,
  useEventsBusForIDE,
} from "../../hooks";
import { useWindowDimensions } from "../../hooks/useWindowDimensions";
import { telemetryApi } from "../../services/refact/telemetry";

import styles from "./Toolbar.module.css";
import { useActiveTeamsGroup } from "../../hooks/useActiveTeamsGroup";

export type DashboardTab = {
  type: "dashboard";
};

function isDashboardTab(tab: Tab): tab is DashboardTab {
  return tab.type === "dashboard";
}

export type ChatTab = {
  type: "chat";
  id: string;
};

function isChatTab(tab: Tab): tab is ChatTab {
  return tab.type === "chat";
}

export type Tab = DashboardTab | ChatTab;

export type ToolbarProps = {
  activeTab: Tab;
};

export const Toolbar = ({ activeTab }: ToolbarProps) => {
  const dispatch = useAppDispatch();
  const tabNav = useRef<HTMLElement | null>(null);
  const [tabNavWidth, setTabNavWidth] = useState(0);
  const { width: windowWidth } = useWindowDimensions();
  const [focus, setFocus] = useState<HTMLElement | null>(null);

  const refs = useTourRefs();
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const openThreadIds = useAppSelector(selectOpenThreadIds);
  const allThreads = useAppSelector(selectAllThreads);
  const currentChatId = useAppSelector(selectChatId);
  const { newChatEnabled } = useActiveTeamsGroup();

  const { openSettings, openHotKeys } = useEventsBusForIDE();

  const [renamingTabId, setRenamingTabId] = useState<string | null>(null);
  const [newTitle, setNewTitle] = useState<string | null>(null);

  const handleNavigation = useCallback(
    (to: DropdownNavigationOptions | "chat") => {
      if (to === "settings") {
        openSettings();
        void sendTelemetryEvent({
          scope: `openSettings`,
          success: true,
          error_message: "",
        });
      } else if (to === "hot keys") {
        openHotKeys();
        void sendTelemetryEvent({
          scope: `openHotkeys`,
          success: true,
          error_message: "",
        });
      } else if (to === "fim") {
        dispatch(push({ name: "fill in the middle debug page" }));
        void sendTelemetryEvent({
          scope: `openDebugFim`,
          success: true,
          error_message: "",
        });
      } else if (to === "stats") {
        dispatch(push({ name: "statistics page" }));
        void sendTelemetryEvent({
          scope: `openStats`,
          success: true,
          error_message: "",
        });
      } else if (to === "restart tour") {
        dispatch(popBackTo({ name: "login page" }));
        dispatch(push({ name: "welcome" }));
        dispatch(restart());
        void sendTelemetryEvent({
          scope: `restartTour`,
          success: true,
          error_message: "",
        });
      } else if (to === "integrations") {
        dispatch(push({ name: "integrations page" }));
        void sendTelemetryEvent({
          scope: `openIntegrations`,
          success: true,
          error_message: "",
        });
      } else if (to === "providers") {
        dispatch(push({ name: "providers page" }));
        void sendTelemetryEvent({
          scope: `openProviders`,
          success: true,
          error_message: "",
        });
      } else if (to === "chat") {
        dispatch(popBackTo({ name: "history" }));
        dispatch(push({ name: "chat" }));
      }
    },
    [dispatch, sendTelemetryEvent, openSettings, openHotKeys],
  );

  const onCreateNewChat = useCallback(() => {
    setRenamingTabId(null);
    dispatch(newChatAction());
    dispatch(clearThreadPauseReasons({ id: currentChatId }));
    dispatch(setThreadConfirmationStatus({ id: currentChatId, wasInteracted: false, confirmationStatus: true }));
    handleNavigation("chat");
    void sendTelemetryEvent({
      scope: `openNewChat`,
      success: true,
      error_message: "",
    });
  }, [dispatch, currentChatId, sendTelemetryEvent, handleNavigation]);

  const goToTab = useCallback(
    (tab: Tab) => {
      if (tab.type === "dashboard") {
        dispatch(popBackTo({ name: "history" }));
      } else {
        dispatch(switchToThread({ id: tab.id }));
        dispatch(popBackTo({ name: "history" }));
        dispatch(push({ name: "chat" }));
      }
      void sendTelemetryEvent({
        scope: `goToTab/${tab.type}`,
        success: true,
        error_message: "",
      });
    },
    [dispatch, sendTelemetryEvent],
  );

  useEffect(() => {
    if (!tabNav.current) {
      return;
    }
    setTabNavWidth(tabNav.current.offsetWidth);
  }, [tabNav, windowWidth]);

  useEffect(() => {
    if (focus === null) return;

    // the function scrollIntoView doesn't always exist, and will crash on unit tests
    // eslint-disable-next-line  @typescript-eslint/no-unnecessary-condition
    if (focus.scrollIntoView) {
      focus.scrollIntoView();
    }
  }, [focus]);

  const tabs = useMemo(() => {
    return openThreadIds
      .map((id) => {
        const runtime = allThreads[id];
        if (!runtime) return null;
        return {
          id,
          title: runtime.thread.title || "New Chat",
          read: runtime.thread.read,
          streaming: runtime.streaming,
        };
      })
      .filter((t): t is NonNullable<typeof t> => t !== null);
  }, [openThreadIds, allThreads]);

  const shouldCollapse = useMemo(() => {
    const dashboardWidth = windowWidth < 400 ? 47 : 70;
    const totalWidth = dashboardWidth + 140 * tabs.length;
    return tabNavWidth < totalWidth;
  }, [tabNavWidth, tabs.length, windowWidth]);

  const handleChatThreadDeletion = useCallback((tabId: string) => {
    dispatch(deleteChatById(tabId));
    dispatch(closeThread({ id: tabId }));
    if (activeTab.type === "chat" && activeTab.id === tabId) {
      goToTab({ type: "dashboard" });
    }
  }, [dispatch, activeTab, goToTab]);

  const handleChatThreadRenaming = useCallback((tabId: string) => {
    setRenamingTabId(tabId);
  }, []);

  const handleKeyUpOnRename = useCallback(
    (event: KeyboardEvent<HTMLInputElement>, tabId: string) => {
      if (event.code === "Escape") {
        setRenamingTabId(null);
      }
      if (event.code === "Enter") {
        setRenamingTabId(null);
        if (!newTitle || newTitle.trim() === "") return;
        dispatch(
          saveTitle({
            id: tabId,
            title: newTitle,
            isTitleGenerated: true,
          }),
        );
        dispatch(updateChatTitleById({ chatId: tabId, newTitle: newTitle }));
      }
    },
    [dispatch, newTitle],
  );

  const handleChatTitleChange = (event: ChangeEvent<HTMLInputElement>) => {
    setNewTitle(event.target.value);
  };

  const handleCloseTab = useCallback((event: MouseEvent, tabId: string) => {
    event.stopPropagation();
    event.preventDefault();
    dispatch(closeThread({ id: tabId }));
    if (activeTab.type === "chat" && activeTab.id === tabId) {
      const remainingTabs = tabs.filter((t) => t.id !== tabId);
      if (remainingTabs.length > 0) {
        goToTab({ type: "chat", id: remainingTabs[0].id });
      } else {
        goToTab({ type: "dashboard" });
      }
    }
  }, [dispatch, activeTab, tabs, goToTab]);

  return (
    <Flex align="center" m="4px" gap="4px" style={{ alignSelf: "stretch" }}>
      <Flex flexGrow="1" align="start" maxHeight="40px" overflowY="hidden">
        <TabNav.Root style={{ flex: 1, overflowX: "scroll" }} ref={tabNav}>
          <TabNav.Link
            active={isDashboardTab(activeTab)}
            ref={(x) => refs.setBack(x)}
            onClick={() => {
              setRenamingTabId(null);
              goToTab({ type: "dashboard" });
            }}
            style={{ cursor: "pointer" }}
          >
            {windowWidth < 400 || shouldCollapse ? <HomeIcon /> : "Home"}
          </TabNav.Link>
          {tabs.map((tab) => {
            const isActive = isChatTab(activeTab) && activeTab.id === tab.id;
            const isRenaming = renamingTabId === tab.id;

            if (isRenaming) {
              return (
                <TextField.Root
                  my="auto"
                  key={tab.id}
                  autoComplete="off"
                  onKeyUp={(e) => handleKeyUpOnRename(e, tab.id)}
                  onBlur={() => setRenamingTabId(null)}
                  autoFocus
                  size="2"
                  defaultValue={tab.title}
                  onChange={handleChatTitleChange}
                  className={styles.RenameInput}
                />
              );
            }
            return (
              <TabNav.Link
                active={isActive}
                key={tab.id}
                onClick={() => goToTab({ type: "chat", id: tab.id })}
                style={{ minWidth: 0, maxWidth: "150px", cursor: "pointer" }}
                ref={isActive ? setFocus : undefined}
                title={tab.title}
              >
                {tab.streaming && <Spinner />}
                {!tab.streaming && tab.read === false && <DotFilledIcon />}
                <Flex gap="2" align="center">
                  <TruncateLeft
                    style={{
                      maxWidth: shouldCollapse ? "25px" : "80px",
                    }}
                  >
                    {tab.title}
                  </TruncateLeft>
                  <Flex gap="1" align="center">
                    <DropdownMenu.Root>
                      <DropdownMenu.Trigger>
                        <IconButton
                          size="1"
                          variant="ghost"
                          color="gray"
                          title="Tab actions"
                          onClick={(e) => e.stopPropagation()}
                        >
                          <DotsVerticalIcon />
                        </IconButton>
                      </DropdownMenu.Trigger>
                      <DropdownMenu.Content
                        size="1"
                        side="bottom"
                        align="end"
                        style={{ minWidth: 110 }}
                      >
                        <DropdownMenu.Item onClick={() => handleChatThreadRenaming(tab.id)}>
                          Rename
                        </DropdownMenu.Item>
                        <DropdownMenu.Item
                          onClick={() => handleChatThreadDeletion(tab.id)}
                          color="red"
                        >
                          Delete chat
                        </DropdownMenu.Item>
                      </DropdownMenu.Content>
                    </DropdownMenu.Root>
                    <IconButton
                      size="1"
                      variant="ghost"
                      color="gray"
                      title="Close tab"
                      onClick={(e) => handleCloseTab(e, tab.id)}
                    >
                      <Cross1Icon />
                    </IconButton>
                  </Flex>
                </Flex>
              </TabNav.Link>
            );
          })}
        </TabNav.Root>
      </Flex>
      {windowWidth < 400 ? (
        <IconButton
          variant="outline"
          ref={(x) => refs.setNewChat(x)}
          onClick={onCreateNewChat}
        >
          <PlusIcon />
        </IconButton>
      ) : (
        <Button
          variant="outline"
          ref={(x) => refs.setNewChat(x)}
          onClick={onCreateNewChat}
          disabled={!newChatEnabled}
        >
          <PlusIcon />
          <Text>New chat</Text>
        </Button>
      )}
      <Dropdown handleNavigation={handleNavigation} />
    </Flex>
  );
};
