<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { IS_WINDOWS } from "$shared/platform";
  import { preferences } from "$domain/preferences";
  import SitesSetApart from "../SitesSetApart.svelte";
  import WindowsPrivacy from "../WindowsPrivacy.svelte";
  import { showPreviews } from "../../lib/settings-model";
  import PreviewNotice from "../PreviewNotice.svelte";
  import ProtectionControls from "../ProtectionControls.svelte";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import PreviewSelect from "../PreviewSelect.svelte";
  import PreviewToggle from "../PreviewToggle.svelte";
  import PreferenceSwitch from "../PreferenceSwitch.svelte";
  import ClearBrowsingData from "../ClearBrowsingData.svelte";
  import SiteData from "../SiteData.svelte";
  import SiteExceptions from "../SiteExceptions.svelte";
  import PasskeysGroup from "../PasskeysGroup.svelte";
  import SiteSettings from "../SiteSettings.svelte";
</script>

<ProtectionControls />
{#if IS_WINDOWS}<WindowsPrivacy />{/if}
{#if IS_WINDOWS}
  <SettingsGroup title={m.privacy_cookie_banners_group()}>
    <PreferenceSwitch id="privacy.cookie-banners" preference="privacy.cookie-banners" />
    {#if preferences.value("privacy.cookie-banners") !== "false"}
      <SitesSetApart preference="privacy.cookie-banners-except" />
    {/if}
  </SettingsGroup>
{/if}
<SettingsGroup title={m.privacy_streamer_group()}>
  <PreferenceSwitch id="privacy.streamer-mode" preference="privacy.streamer-mode" />
  {#if IS_WINDOWS}
    <PreferenceSwitch id="privacy.streamer-auto" preference="privacy.streamer-auto" />
  {/if}
</SettingsGroup>
<PasskeysGroup />
<SettingsGroup title={m.site_settings_title()} description={m.site_settings_help()}>
  <SiteSettings />
</SettingsGroup>
{#if showPreviews}
  <PreviewNotice />
  <SettingsGroup title={m.settings_site_permissions()}
    ><PreviewSelect id="privacy.camera" /><PreviewSelect id="privacy.microphone" /><PreviewSelect
      id="privacy.location"
    /><PreviewSelect id="privacy.notifications" /></SettingsGroup
  >
  <SettingsGroup title={m.privacy_media_group()}
    ><PreviewSelect id="privacy.popups" /><PreviewSelect id="privacy.clipboard" /><PreviewSelect
      id="privacy.automatic-downloads"
    /><PreviewSelect id="privacy.sound" /></SettingsGroup
  >
  <SiteExceptions />
{/if}
<SettingsGroup title={m.settings_browsing_data()}>
  {#if showPreviews}<PreviewToggle id="privacy.cleanup" />{/if}
  <ClearBrowsingData />
  <SiteData />
</SettingsGroup>
