import { expect, test, vi } from "vitest";
import { render } from "vitest-browser-svelte";
import "$styles/global.css";
import * as tools from "$session/tools.svelte";
import AskButton from "../components/AskButton.svelte";

vi.mock("$shared/ipc/bindings", async () => {
  const { mockBindings } = await import("$shared/testing/bindings");
  return mockBindings({
    settingGet: async () => null,
    sidebarSetHidden: async () => {},
    sidebarPeek: async () => {},
  });
});

test("the assistant is one press away at the foot of the column, and a second press closes it", async () => {
  tools.close();
  const screen = await render(AskButton);
  const button = screen.getByRole("button", { name: "Ask Aven" });
  await expect.element(button).toBeVisible();
  await expect.element(button).toHaveAttribute("aria-pressed", "false");

  await button.click();
  expect(tools.activeTool()).toBe("ai");
  await expect.element(button).toHaveAttribute("aria-pressed", "true");

  await button.click();
  expect(tools.activeTool()).toBeNull();
  await expect.element(button).toHaveAttribute("aria-pressed", "false");
});

test("down the rail it is a little smaller, as the avatar is", async () => {
  const screen = await render(AskButton, { compact: true });
  const button = screen.getByRole("button", { name: "Ask Aven" }).element() as HTMLElement;
  expect(button.getBoundingClientRect().width).toBe(36);
});
