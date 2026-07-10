<script lang="ts">
  import type { Tab } from "@runyard/common";
  import EditorTab from "./EditorTab.svelte";
  import ExplorerTab from "./ExplorerTab.svelte";
  import SettingsPanel from "./SettingsPanel.svelte";
  import WelcomePanel from "./WelcomePanel.svelte";
  import PlaceholderTab from "./PlaceholderTab.svelte";
  import TerminalPanel from "./TerminalPanel.svelte";
  import GitPanel from "./GitPanel.svelte";

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
{:else if tab.type === "settings"}
  <SettingsPanel />
{:else if tab.type === "welcome"}
  <WelcomePanel />
{:else}
  <PlaceholderTab {tab} />
{/if}
