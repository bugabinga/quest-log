/**
 * E2E Tests for Auth Modal
 *
 * Tests the login flow:
 * 1. Empty password -> submit button disabled
 * 2. Wrong password -> error message shown
 * 3. Correct password -> modal closes, editor shown
 */

import {
  clearBrowserState,
  launchBrowser,
  newPage,
  typeIntoInput,
} from "./browser.ts";
import { assertEquals } from "@std/assert";
import type { Page } from "puppeteer-core";

async function gotoEditor(page: Page): Promise<void> {
  await page.goto("http://localhost:3000/editor");
  await page.waitForSelector("#auth-modal", { visible: true });
}

Deno.test({
  name: "Auth Modal - Submit button should be disabled when password is empty",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await clearBrowserState(page);
      await gotoEditor(page);

      const isDisabled = await page.$eval(
        ".auth-submit",
        (btn) => (btn as unknown as { disabled: boolean }).disabled,
      );
      assertEquals(
        isDisabled,
        true,
        "Submit button should be disabled when password is empty",
      );
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "Auth Modal - No JavaScript errors on page load",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await clearBrowserState(page);
      const errors: string[] = [];

      page.on("console", (msg) => {
        if (msg.type() === "error") {
          errors.push(msg.text());
        }
      });

      await gotoEditor(page);

      const actualErrors = errors.filter((e) =>
        !e.includes("favicon") &&
        !e.includes("404") &&
        !e.includes("view-transition")
      );

      assertEquals(
        actualErrors.length,
        0,
        `Expected no JS errors, got: ${actualErrors.join(", ")}`,
      );
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "Auth Modal - Error message should display for wrong password",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await clearBrowserState(page);
      await gotoEditor(page);

      await typeIntoInput(page, "#password", "wrongpassword");
      await page.click(".auth-submit");

      await page.waitForFunction(
        () =>
          document.querySelector(".auth-error")?.textContent?.includes(
            "Invalid password",
          ),
        { timeout: 5000 },
      );

      const errorText = await page.$eval(".auth-error", (el) => el.textContent);
      assertEquals(
        errorText?.includes("Invalid password"),
        true,
        `Expected error message, got: ${errorText}`,
      );
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name:
    "Auth Modal - Modal should close and editor should show for correct password",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await clearBrowserState(page);
      await gotoEditor(page);

      await typeIntoInput(page, "#password", "dev");
      await page.click(".auth-submit");

      await page.waitForFunction(
        () => document.querySelector(".editor-container") !== null,
        { timeout: 5000 },
      );

      const editorVisible = await page.$(".editor-container");
      assertEquals(
        editorVisible !== null,
        true,
        "Editor should be visible after login",
      );
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "Auth Modal - No JavaScript errors after successful login",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await clearBrowserState(page);
      const errors: string[] = [];

      page.on("console", (msg) => {
        if (msg.type() === "error") {
          errors.push(msg.text());
        }
      });

      await gotoEditor(page);
      await typeIntoInput(page, "#password", "dev");
      await page.click(".auth-submit");

      await page.waitForSelector(".editor-container");
      await page.waitForSelector(".editor-tabs");

      const realErrors = errors.filter((e) =>
        !e.includes("favicon") &&
        !e.includes("404") &&
        !e.includes("view-transition") &&
        !e.includes("_activeTab")
      );

      assertEquals(
        realErrors.length,
        0,
        `Expected no JS errors after login, got: ${realErrors.join(", ")}`,
      );
    } finally {
      await browser.close();
    }
  },
});
