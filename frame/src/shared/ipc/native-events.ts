import type {
  WorkEnvironmentChanged,
  WorkChanged,
  WorkHumanChanged,
  WorkDecisionPreferenceChanged,
  WorkModelsChanged,
  AccountChanged,
  PanelState,
  PageAssistChanged,
  ProfilesView,
  FaviconsView,
  NoteOpenRequested,
  NoteChanges,
  ResourceChanged,
  DownloadsChanged,
  BlockerStatusChanged,
  BrowserCredentialCapabilityChanged,
  FocusChanged,
  ExtensionActionFailed,
  ExtensionActionShortcut,
  ExtensionActionsChanged,
  FindResultView,
  ImportJobView,
  ItemsChanged,
  KeymapChanged,
  LayoutChanged,
  MediaPlayerChanged,
  OperationProcessed,
  PagePermissionPromptChanged,
  RuntimeStatusChanged,
  SearchChanged,
  TabChanged,
  UiCommand,
  VoiceChanged,
  VoiceLevels,
  SwipeScroll,
  WebExtensionAccessRequested,
  WebExtensionStoreClicked,
} from "./bindings";

type PayloadEvent<T> = { payload: T };
type Listener<T> = (event: PayloadEvent<T>) => void;
type PresentationTab = { tab: TabChanged; active: string | null };

function scopedEvent<T>(name: string) {
  return {
    listen(listener: Listener<T>): Promise<() => void> {
      const handler = (event: Event) => listener({ payload: (event as CustomEvent<T>).detail });
      window.addEventListener(name, handler);
      return Promise.resolve(() => window.removeEventListener(name, handler));
    },
  };
}

// Rust delivers these directly into the intended WebView. Do not replace this
// with @tauri-apps/api/event: its listen command accepts a caller-selected
// target, which would let the launcher panel subscribe to main-window state.
export const nativeEventNames = {
  windowsReputationManaged: "aven:windows-reputation-managed",
  workEnvironmentChanged: "aven:work-environment-changed",
  workChanged: "aven:work-changed",
  workHumanChanged: "aven:work-human-changed",
  workDecisionPreferenceChanged: "aven:work-decision-preference-changed",
  workModelsChanged: "aven:work-models-changed",
  pageAssistChanged: "aven:page-assist-changed",
  accountChanged: "aven:account-changed",
  panelState: "aven:panel-state",
  favicons: "aven:favicons",
  noteOpenRequested: "aven:note-open-requested",
  resourceChanged: "aven:resource-changed",
  notesChanged: "aven:notes-changed",
  downloadsChanged: "aven:downloads-changed",
  resourceClose: "aven:resource-close",
  resourceCloseCancelled: "aven:resource-close-cancelled",
  browserCredentialCapabilityChanged: "aven:browser-credential-capability",
  itemsChanged: "aven:items",
  tabChanged: "aven:tab",
  extensionActionsChanged: "aven:extension-actions",
  extensionActionFailed: "aven:extension-action-failed",
  extensionActionShortcut: "aven:extension-action-shortcut",
  pagePermissionPromptChanged: "aven:page-permission-prompt",
  webExtensionAccessRequested: "aven:web-extension-access",
  webExtensionStoreClicked: "aven:web-extension-store-click",
  webExtensionDropped: "aven:web-extension-dropped",
  browserReturn: "aven:browser-return",
  presentationTab: "aven:presentation-tab",
  uiCommand: "aven:ui-command",
  keymapChanged: "aven:keymap-changed",
  importProgress: "aven:import-progress",
  find: "aven:find",
  searchChanged: "aven:search",
  layoutChanged: "aven:layout",
  mediaPlayerChanged: "aven:media-player",
  operationProcessed: "aven:operation-processed",
  runtimeStatusChanged: "aven:runtime-status",
  blockerStatusChanged: "aven:blocker-status",
  focusChanged: "aven:focus",
  voiceChanged: "aven:voice-changed",
  voiceLevels: "aven:voice-levels",
  swipeScroll: "aven:swipe",
  profilesChanged: "aven:profiles",
} as const;

export const events = {
  windowsReputationManaged: scopedEvent<boolean>(nativeEventNames.windowsReputationManaged),
  workEnvironmentChanged: scopedEvent<WorkEnvironmentChanged>(
    nativeEventNames.workEnvironmentChanged,
  ),
  workChanged: scopedEvent<WorkChanged>(nativeEventNames.workChanged),
  workHumanChanged: scopedEvent<WorkHumanChanged>(nativeEventNames.workHumanChanged),
  workDecisionPreferenceChanged: scopedEvent<WorkDecisionPreferenceChanged>(
    nativeEventNames.workDecisionPreferenceChanged,
  ),
  workModelsChanged: scopedEvent<WorkModelsChanged>(nativeEventNames.workModelsChanged),
  pageAssistChanged: scopedEvent<PageAssistChanged>(nativeEventNames.pageAssistChanged),
  accountChanged: scopedEvent<AccountChanged>(nativeEventNames.accountChanged),
  favicons: scopedEvent<FaviconsView>(nativeEventNames.favicons),
  noteOpenRequested: scopedEvent<NoteOpenRequested>(nativeEventNames.noteOpenRequested),
  resourceClose: scopedEvent<string>(nativeEventNames.resourceClose),
  resourceCloseCancelled: scopedEvent<string>(nativeEventNames.resourceCloseCancelled),
  resourceChanged: scopedEvent<ResourceChanged>(nativeEventNames.resourceChanged),
  notesChanged: scopedEvent<NoteChanges>(nativeEventNames.notesChanged),
  downloadsChanged: scopedEvent<DownloadsChanged>(nativeEventNames.downloadsChanged),
  panelState: scopedEvent<PanelState>(nativeEventNames.panelState),
  browserCredentialCapabilityChanged: scopedEvent<BrowserCredentialCapabilityChanged>(
    nativeEventNames.browserCredentialCapabilityChanged,
  ),
  itemsChanged: scopedEvent<ItemsChanged>(nativeEventNames.itemsChanged),
  tabChanged: scopedEvent<TabChanged>(nativeEventNames.tabChanged),
  extensionActionsChanged: scopedEvent<ExtensionActionsChanged>(
    nativeEventNames.extensionActionsChanged,
  ),
  extensionActionFailed: scopedEvent<ExtensionActionFailed>(nativeEventNames.extensionActionFailed),
  extensionActionShortcut: scopedEvent<ExtensionActionShortcut>(
    nativeEventNames.extensionActionShortcut,
  ),
  pagePermissionPromptChanged: scopedEvent<PagePermissionPromptChanged>(
    nativeEventNames.pagePermissionPromptChanged,
  ),
  webExtensionAccessRequested: scopedEvent<WebExtensionAccessRequested>(
    nativeEventNames.webExtensionAccessRequested,
  ),
  webExtensionStoreClicked: scopedEvent<WebExtensionStoreClicked>(
    nativeEventNames.webExtensionStoreClicked,
  ),
  /** The path of an extension package dropped on the browser. */
  webExtensionDropped: scopedEvent<string>(nativeEventNames.webExtensionDropped),
  browserReturn: scopedEvent<ItemsChanged>(nativeEventNames.browserReturn),
  presentationTab: scopedEvent<PresentationTab>(nativeEventNames.presentationTab),
  uiCommand: scopedEvent<UiCommand>(nativeEventNames.uiCommand),
  keymapChanged: scopedEvent<KeymapChanged>(nativeEventNames.keymapChanged),
  importProgress: scopedEvent<ImportJobView>(nativeEventNames.importProgress),
  find: scopedEvent<FindResultView>(nativeEventNames.find),
  searchChanged: scopedEvent<SearchChanged>(nativeEventNames.searchChanged),
  layoutChanged: scopedEvent<LayoutChanged>(nativeEventNames.layoutChanged),
  mediaPlayerChanged: scopedEvent<MediaPlayerChanged>(nativeEventNames.mediaPlayerChanged),
  operationProcessed: scopedEvent<OperationProcessed>(nativeEventNames.operationProcessed),
  runtimeStatusChanged: scopedEvent<RuntimeStatusChanged>(nativeEventNames.runtimeStatusChanged),
  blockerStatusChanged: scopedEvent<BlockerStatusChanged>(nativeEventNames.blockerStatusChanged),
  focusChanged: scopedEvent<FocusChanged>(nativeEventNames.focusChanged),
  voiceChanged: scopedEvent<VoiceChanged>(nativeEventNames.voiceChanged),
  voiceLevels: scopedEvent<VoiceLevels>(nativeEventNames.voiceLevels),
  swipeScroll: scopedEvent<SwipeScroll>(nativeEventNames.swipeScroll),
  profilesChanged: scopedEvent<ProfilesView>(nativeEventNames.profilesChanged),
};
