<script lang="ts">
  import type { Tab } from "@runyard/common";
  import EditorTab from "./EditorTab.svelte";
  import ExplorerTab from "./ExplorerTab.svelte";
  import SettingsPanel from "./SettingsPanel.svelte";
  import WelcomePanel from "./WelcomePanel.svelte";
  import PlaceholderTab from "./PlaceholderTab.svelte";
  import TerminalPanel from "./TerminalPanel.svelte";
  import GitPanel from "./GitPanel.svelte";
  import ChatPanel from "./ChatPanel.svelte";
  import AgentManagerPanel from "./AgentManagerPanel.svelte";
  import McpManagerPanel from "./McpManagerPanel.svelte";
  import SkillsPanel from "./SkillsPanel.svelte";
  import AgentTasksPanel from "./AgentTasksPanel.svelte";
  import NotesPanel from "./NotesPanel.svelte";
  import TodoPanel from "./TodoPanel.svelte";
  import DiffViewerPanel from "./DiffViewerPanel.svelte";

  let { tab } = $props<{ tab: Tab }>();
</script>

{#if tab.type === "editor"}
  <EditorTab {tab} />
{:else if tab.type === "explorer"}
  <ExplorerTab {tab} />
{:else if tab.type === "terminal"}
  <TerminalPanel
    terminalId={tab.props.terminalId as string}
    cwd={tab.props.cwd as string | undefined}
  />
{:else if tab.type === "git"}
  <GitPanel
    workspacePath={tab.props.workspacePath as string ?? "../../"}
  />
{:else if tab.type === "chat"}
  <ChatPanel {tab} />
{:else if tab.type === "settings"}
  <SettingsPanel />
{:else if tab.type === "agent-manager"}
  <AgentManagerPanel />
{:else if tab.type === "mcp-manager"}
  <McpManagerPanel />
{:else if tab.type === "skills"}
  <SkillsPanel />
{:else if tab.type === "agent-tasks"}
  <AgentTasksPanel />
{:else if tab.type === "notes"}
  <NotesPanel workspacePath={tab.props.workspacePath as string ?? "../../"} />
{:else if tab.type === "todo"}
  <TodoPanel workspacePath={tab.props.workspacePath as string ?? "../../"} />
{:else if tab.type === "diff-viewer"}
  <DiffViewerPanel />
{:else if tab.type === "welcome"}
  <WelcomePanel />
{:else}
  <PlaceholderTab {tab} />
{/if}
