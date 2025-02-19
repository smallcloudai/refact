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
  DotFilledIcon,
  DotsVerticalIcon,
  HomeIcon,
  PlusIcon,
} from "@radix-ui/react-icons";
// import { newChatAction } from "../../events";
import { restart, useTourRefs } from "../../features/Tour";
// import { popBackTo, push } from "../../features/Pages/pagesSlice";
import {
  ChangeEvent,
  KeyboardEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  deleteChatById,
  getHistory,
  updateChatTitleById,
} from "../../features/History/historySlice";
import {
  // restoreChat,
  saveTitle,
  selectThread,
} from "../../features/Chat";
import { TruncateLeft } from "../Text";
import {
  useAppDispatch,
  useAppSelector,
  useEventsBusForIDE,
} from "../../hooks";
import { useWindowDimensions } from "../../hooks/useWindowDimensions";
import { clearPauseReasonsAndHandleToolsStatus } from "../../features/ToolConfirmation/confirmationSlice";
import { telemetryApi } from "../../services/refact/telemetry";
import { useNavigate } from "react-router";
import styles from "./Toolbar.module.css";

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
  // TODO: handle active tabs
  activeTab?: Tab;
};

// TODO: remove active tabs
export const Toolbar = ({ activeTab }: ToolbarProps) => {
  const navigate = useNavigate();
  const dispatch = useAppDispatch();
  const tabNav = useRef<HTMLElement | null>(null);
  const [tabNavWidth, setTabNavWidth] = useState(0);
  const { width: windowWidth } = useWindowDimensions();
  const [focus, setFocus] = useState<HTMLElement | null>(null);

  const refs = useTourRefs();
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const history = useAppSelector(getHistory, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const isStreaming = useAppSelector((app) => app.chat.streaming);
  const { isTitleGenerated, id: chatId } = useAppSelector(selectThread);
  const cache = useAppSelector((app) => app.chat.cache);
  const { openSettings, openHotKeys } = useEventsBusForIDE();

  const [isOnlyOneChatTab, setIsOnlyOneChatTab] = useState(false);
  const [isRenaming, setIsRenaming] = useState(false);
  const [newTitle, setNewTitle] = useState<string | null>(null);

  const shouldChatTabLinkBeNotClickable = useMemo(() => {
    return isOnlyOneChatTab && activeTab && !isDashboardTab(activeTab);
  }, [isOnlyOneChatTab, activeTab]);

  // TODO: handle router nav here (refactor later to use Link)
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
        // dispatch(push({ name: "fill in the middle debug page" }));
        void navigate("/fim");
        void sendTelemetryEvent({
          scope: `openDebugFim`,
          success: true,
          error_message: "",
        });
      } else if (to === "stats") {
        // dispatch(push({ name: "statistics page" }));
        void navigate("/statistics");
        void sendTelemetryEvent({
          scope: `openStats`,
          success: true,
          error_message: "",
        });
      } else if (to === "restart tour") {
        // dispatch(popBackTo({ name: "login page" }));
        // dispatch(push({ name: "welcome" }));
        void navigate("/welcome");
        dispatch(restart());
        void sendTelemetryEvent({
          scope: `restartTour`,
          success: true,
          error_message: "",
        });
      } else if (to === "integrations") {
        // dispatch(push({ name: "integrations page" }));
        void navigate("/integrations");
        void sendTelemetryEvent({
          scope: `openIntegrations`,
          success: true,
          error_message: "",
        });
      } else if (to === "chat") {
        // dispatch(popBackTo({ name: "history" }));
        // dispatch(push({ name: "chat" }));
        // dispatch(newChatAction());
        void navigate("/chat");
      } else if (to === "knowledge list") {
        // TODO: send telemetry
        void navigate("/knowledge");
      }
    },
    [openSettings, sendTelemetryEvent, openHotKeys, navigate, dispatch],
  );

  const onCreateNewChat = useCallback(() => {
    setIsRenaming((prev) => (prev ? !prev : prev));
    // dispatch(newChatAction());
    dispatch(
      clearPauseReasonsAndHandleToolsStatus({
        wasInteracted: false,
        confirmationStatus: true,
      }),
    );
    handleNavigation("chat");
    void sendTelemetryEvent({
      scope: `openNewChat`,
      success: true,
      error_message: "",
    });
  }, [dispatch, sendTelemetryEvent, handleNavigation]);

  const goToTab = useCallback(
    (tab: Tab) => {
      if (tab.type === "dashboard") {
        void navigate("/");
        // dispatch(popBackTo({ name: "history" }));
        // dispatch(newChatAction());
      } else {
        // if (shouldChatTabLinkBeNotClickable) return;
        // const chat = history.find((chat) => chat.id === tab.id);
        // if (chat != undefined) {
        //   dispatch(restoreChat(chat));
        // }
        // dispatch(popBackTo({ name: "history" }));
        // dispatch(push({ name: "chat" }));
        void navigate(`/chat/${tab.id}`);
      }
      void sendTelemetryEvent({
        scope: `goToTab/${tab.type}`,
        success: true,
        error_message: "",
      });
    },
    [sendTelemetryEvent, navigate],
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
    return history.filter(
      (chat) =>
        chat.read === false ||
        (activeTab?.type === "chat" && activeTab.id == chat.id),
    );
  }, [history, activeTab]);

  const shouldCollapse = useMemo(() => {
    const dashboardWidth = windowWidth < 400 ? 47 : 70; // todo: compute this
    const totalWidth = dashboardWidth + 140 * tabs.length;
    return tabNavWidth < totalWidth;
  }, [tabNavWidth, tabs.length, windowWidth]);

  const handleChatThreadDeletion = useCallback(() => {
    dispatch(deleteChatById(chatId));
    // goToTab({ type: "dashboard" });
    void navigate("/");
  }, [dispatch, chatId, navigate]);

  const handleChatThreadRenaming = useCallback(() => {
    setIsRenaming(true);
  }, []);

  const handleKeyUpOnRename = useCallback(
    (event: KeyboardEvent<HTMLInputElement>) => {
      if (event.code === "Escape") {
        setIsRenaming(false);
      }
      if (event.code === "Enter") {
        setIsRenaming(false);
        if (!newTitle || newTitle.trim() === "") return;
        if (!isTitleGenerated) {
          dispatch(
            saveTitle({
              id: chatId,
              title: newTitle,
              isTitleGenerated: true,
            }),
          );
        }
        dispatch(updateChatTitleById({ chatId: chatId, newTitle: newTitle }));
      }
    },
    [dispatch, newTitle, chatId, isTitleGenerated],
  );

  const handleChatTitleChange = (event: ChangeEvent<HTMLInputElement>) => {
    setNewTitle(event.target.value);
  };

  useEffect(() => {
    setIsOnlyOneChatTab(tabs.length < 2);
  }, [tabs]);

  return (
    <Flex align="center" m="4px" gap="4px" style={{ alignSelf: "stretch" }}>
      <Flex flexGrow="1" align="start" maxHeight="40px" overflowY="hidden">
        <TabNav.Root style={{ flex: 1, overflowX: "scroll" }} ref={tabNav}>
          <TabNav.Link
            active={activeTab && isDashboardTab(activeTab)}
            ref={(x) => refs.setBack(x)}
            onClick={() => {
              setIsRenaming((prev) => (prev ? !prev : prev));
              goToTab({ type: "dashboard" });
            }}
            style={{ cursor: "pointer" }}
          >
            {windowWidth < 400 || shouldCollapse ? <HomeIcon /> : "Home"}
          </TabNav.Link>
          {tabs.map((chat) => {
            const isStreamingThisTab =
              chat.id in cache ||
              (activeTab &&
                isChatTab(activeTab) &&
                chat.id === activeTab.id &&
                isStreaming);
            const isActive =
              activeTab && isChatTab(activeTab) && activeTab.id == chat.id;
            if (isRenaming) {
              return (
                <TextField.Root
                  my="auto"
                  key={chat.id}
                  autoComplete="off"
                  onKeyUp={handleKeyUpOnRename}
                  onBlur={() => setIsRenaming(false)}
                  autoFocus
                  size="2"
                  defaultValue={isTitleGenerated ? chat.title : ""}
                  onChange={handleChatTitleChange}
                  className={styles.RenameInput}
                />
              );
            }
            return (
              <TabNav.Link
                active={isActive}
                key={chat.id}
                onClick={() => {
                  if (shouldChatTabLinkBeNotClickable) return;
                  goToTab({ type: "chat", id: chat.id });
                }}
                style={{ minWidth: 0, maxWidth: "150px", cursor: "pointer" }}
                ref={isActive ? setFocus : undefined}
                title={chat.title}
              >
                {isStreamingThisTab && <Spinner />}
                {!isStreamingThisTab && chat.read === false && (
                  <DotFilledIcon />
                )}
                <Flex gap="2" align="center">
                  <TruncateLeft
                    style={{
                      maxWidth: shouldCollapse ? "25px" : "110px",
                    }}
                  >
                    {chat.title}
                  </TruncateLeft>
                  {isActive && !isStreamingThisTab && isOnlyOneChatTab && (
                    <DropdownMenu.Root>
                      <DropdownMenu.Trigger>
                        <IconButton
                          size="1"
                          variant="ghost"
                          color="gray"
                          title="Title actions"
                        >
                          <DotsVerticalIcon />
                        </IconButton>
                      </DropdownMenu.Trigger>
                      <DropdownMenu.Content
                        size="1"
                        side="bottom"
                        align="end"
                        style={{
                          minWidth: 110,
                        }}
                      >
                        <DropdownMenu.Item onClick={handleChatThreadRenaming}>
                          Rename
                        </DropdownMenu.Item>
                        <DropdownMenu.Item
                          onClick={handleChatThreadDeletion}
                          color="red"
                        >
                          Delete chat
                        </DropdownMenu.Item>
                      </DropdownMenu.Content>
                    </DropdownMenu.Root>
                  )}
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
        >
          <PlusIcon />
          <Text>New chat</Text>
        </Button>
      )}
      <Dropdown handleNavigation={handleNavigation} />
    </Flex>
  );
};
