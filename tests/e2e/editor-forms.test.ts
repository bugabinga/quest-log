import { launchBrowser, newPage, typeIntoInput } from "./browser.ts";

async function login(page: import("puppeteer-core").Page): Promise<void> {
  await page.goto("http://localhost:3000/editor");
  await page.waitForSelector("#auth-modal", { visible: true });
  await typeIntoInput(page, "#password", "dev");
  await page.click(".auth-submit");
  await page.waitForSelector(".editor-container", { visible: true });
}

async function clickButton(
  page: import("puppeteer-core").Page,
  label: string,
): Promise<void> {
  await page.$$eval("button", (buttons, label) => {
    const button = buttons.find((element) =>
      element.textContent?.includes(label)
    );
    if (!(button instanceof HTMLButtonElement)) {
      throw new Error(`Missing button: ${label}`);
    }
    button.click();
  }, label);
}

async function waitForButtonDisabled(
  page: import("puppeteer-core").Page,
  label: string,
  disabled: boolean,
): Promise<void> {
  await page.waitForFunction(
    (label, disabled) => {
      const button = [...document.querySelectorAll("button")].find((element) =>
        element.textContent?.includes(label)
      );
      return button instanceof HTMLButtonElement &&
        button.disabled === disabled;
    },
    { timeout: 3_000 },
    label,
    disabled,
  );
}

async function verifyRequiredTitleBinding(
  page: import("puppeteer-core").Page,
  inputSelector: string,
  saveLabel: string,
  title: string,
): Promise<void> {
  await typeIntoInput(page, inputSelector, title);
  await waitForButtonDisabled(page, saveLabel, false);

  await page.focus(inputSelector);
  await page.keyboard.down("Control");
  await page.keyboard.press("A");
  await page.keyboard.up("Control");
  await page.keyboard.press("Backspace");
  await waitForButtonDisabled(page, saveLabel, true);
}

Deno.test({
  name: "Quest title controls Save Quest validation",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await login(page);
      await clickButton(page, "Add Quest");
      await page.waitForSelector("#quest-title", { visible: true });
      await verifyRequiredTitleBinding(
        page,
        "#quest-title",
        "Save Quest",
        "QA Chrome Quest",
      );
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "Reward title controls Save Reward validation",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await login(page);
      await clickButton(page, "Rewards");
      await page.waitForSelector("#rewards-panel", { visible: true });
      await clickButton(page, "Add Reward");
      await page.waitForSelector("#reward-title", { visible: true });
      await verifyRequiredTitleBinding(
        page,
        "#reward-title",
        "Save Reward",
        "QA Chrome Reward",
      );
    } finally {
      await browser.close();
    }
  },
});
