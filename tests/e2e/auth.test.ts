import { assertEquals } from "@std/assert";
import { clearBrowserState, launchBrowser, newPage } from "./browser.ts";

async function gotoEditor(page: import("puppeteer-core").Page): Promise<void> {
  await page.goto("http://localhost:3000/editor");
  await page.waitForSelector("#auth-modal", { visible: true });
}

Deno.test({
  name: "Editor auth modal renders",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await clearBrowserState(page);
      await gotoEditor(page);

      const heading = await page.$eval(
        "#auth-modal h2",
        (el) => el.textContent ?? "",
      );
      const hasPasswordInput = await page.$("#password") !== null;
      const hasSubmit = await page.$(".auth-submit") !== null;

      assertEquals(heading.includes("Guild Entrance"), true);
      assertEquals(hasPasswordInput, true);
      assertEquals(hasSubmit, true);
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "Editor login reaches editor UI",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      const errors: string[] = [];
      const requests: Array<Record<string, string | number>> = [];
      page.on("console", (message) => {
        if (message.type() === "error") errors.push(message.text());
      });
      page.on("pageerror", (error) => errors.push(String(error)));
      page.on("request", (request) => {
        if (new URL(request.url()).pathname === "/editor/login") {
          requests.push({
            event: "request",
            method: request.method(),
            resourceType: request.resourceType(),
          });
        }
      });
      page.on("requestfailed", (request) => {
        if (new URL(request.url()).pathname === "/editor/login") {
          requests.push({
            event: "requestfailed",
            error: request.failure()?.errorText ?? "unknown",
          });
        }
      });
      page.on("response", (response) => {
        if (new URL(response.url()).pathname === "/editor/login") {
          requests.push({
            event: "response",
            status: response.status(),
            contentType: response.headers()["content-type"] ?? "missing",
          });
        }
      });

      await clearBrowserState(page);
      await gotoEditor(page);
      await page.focus("#password");
      await page.keyboard.type("dev");
      const before = await page.$eval("#login-form", (form) => {
        const input = form.querySelector<HTMLInputElement>("#password");
        const button = form.querySelector<HTMLButtonElement>(".auth-submit");
        const rect = button?.getBoundingClientRect();
        const hit = rect
          ? document.elementFromPoint(
            rect.left + rect.width / 2,
            rect.top + rect.height / 2,
          )
          : null;
        const events: string[] = [];
        button?.addEventListener("click", () => events.push("button-click"));
        form.addEventListener("submit", () => events.push("form-submit"));
        Object.assign(globalThis, { loginTestEvents: events });
        return {
          buttonDisabled: button?.disabled ?? null,
          inputLength: input?.value.length ?? null,
          binding: input?.getAttribute("data-bind") ?? null,
          submit: form.getAttribute("data-on:submit__prevent"),
          hitTarget: hit instanceof Element
            ? `${hit.tagName.toLowerCase()}#${hit.id}.${hit.className}`
            : null,
          videoDisplay: getComputedStyle(
            document.querySelector<HTMLElement>("#video-modal")!,
          ).display,
        };
      });
      await page.click(".auth-submit");
      try {
        await page.waitForSelector(".editor-container", { visible: true });
      } catch (error) {
        const after = await page.evaluate(() => ({
          authError: document.querySelector(".auth-error")?.textContent?.trim(),
          buttonDisabled: (document.querySelector(".auth-submit") as
            | HTMLButtonElement
            | null)?.disabled ?? null,
          inputLength: (document.querySelector("#password") as
            | HTMLInputElement
            | null)?.value.length ?? null,
          path: location.pathname,
          events: (globalThis as typeof globalThis & {
            loginTestEvents?: string[];
          }).loginTestEvents ?? [],
        }));
        throw new Error(
          `Login failed: ${
            JSON.stringify({ before, after, errors, requests })
          }; ${error}`,
        );
      }
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "Editor auth modal has no page-load JavaScript errors",
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
      page.on("pageerror", (error) => {
        errors.push(error instanceof Error ? error.message : String(error));
      });

      await gotoEditor(page);

      const actualErrors = errors.filter((error) =>
        !error.includes("favicon") &&
        !error.includes("404") &&
        !error.includes("view-transition")
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
