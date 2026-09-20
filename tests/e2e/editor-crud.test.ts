import { assert, assertEquals } from "@std/assert";
import type { HTTPResponse, Page } from "puppeteer-core";
import { launchBrowser, newPage } from "./browser.ts";

const BASE_URL = "http://localhost:3000";
const WAIT = 5_000;

type ResponseInfo = { status: number; body: string; requestBody?: string };
type Item = Record<string, unknown>;
type Kind = "quest" | "reward";

const editor = {
  quest: {
    path: "quests",
    panel: "#quests-panel",
    title: "#quest-title",
    description: "#quest-description",
    exp: "#quest-exp",
    image: "#quest-image",
  },
  reward: {
    path: "rewards",
    panel: "#rewards-panel",
    title: "#reward-title",
    description: "#reward-description",
    exp: "#reward-exp",
    image: "#reward-image",
  },
} as const;

function watchBrowserErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  return errors;
}

function assertNoBrowserErrors(
  errors: string[],
  expectedStatuses: number[],
): void {
  const actual = errors.filter((error) =>
    !error.toLowerCase().includes("favicon") &&
    !expectedStatuses.some((status) => error.includes(`status of ${status}`))
  );
  assertEquals(actual, [], `Unexpected browser errors: ${actual.join(" | ")}`);
}

async function fill(
  page: Page,
  selector: string,
  value: string,
): Promise<void> {
  await page.click(selector);
  await page.keyboard.down("Control");
  await page.keyboard.press("A");
  await page.keyboard.up("Control");
  await page.keyboard.type(value);
}

async function clickText(
  page: Page,
  selector: string,
  text: string,
): Promise<void> {
  await page.$$eval(
    selector,
    (elements, expected) => {
      const element = elements.find((candidate) =>
        candidate.textContent?.includes(expected as string)
      );
      if (!(element instanceof HTMLElement)) {
        throw new Error(`Missing element containing ${expected}`);
      }
      element.click();
    },
    text,
  );
}

async function waitForImagePayloadReady(
  page: Page,
  kind: Kind,
): Promise<void> {
  const probeId = `${kind}-image-signal-probe`;
  await page.evaluate(({ probeId, kind }) => {
    const probe = document.createElement("output");
    probe.id = probeId;
    probe.hidden = true;
    probe.setAttribute("data-text", `$_${kind}Image.length`);
    document.body.append(probe);
  }, { probeId, kind });
  await page.waitForFunction(
    (id) => document.getElementById(id)?.textContent === "1",
    { timeout: WAIT },
    probeId,
  );
}

async function waitForRow(
  page: Page,
  kind: Kind,
  title: string,
  present: boolean,
): Promise<void> {
  await page.waitForFunction(
    ({ selector, title, present }) => {
      const found = [...document.querySelectorAll(`${selector} tbody tr`)].some(
        (row) => row.textContent?.includes(title),
      );
      return found === present;
    },
    { timeout: WAIT },
    { selector: editor[kind].panel, title, present },
  );
}

async function clickRowAction(
  page: Page,
  kind: Kind,
  title: string,
  action: "Edit" | "Delete",
): Promise<void> {
  await page.$$eval(
    `${editor[kind].panel} tbody tr`,
    (rows, args) => {
      const row = rows.find((candidate) =>
        candidate.textContent?.includes(args.title)
      );
      const button = [...(row?.querySelectorAll("button") ?? [])].find((
        candidate,
      ) => candidate.textContent?.includes(args.action));
      if (!(button instanceof HTMLButtonElement)) {
        throw new Error(`Missing ${args.action} action for ${args.title}`);
      }
      button.click();
    },
    { title, action },
  );
}

function responseFor(
  page: Page,
  method: string,
  path: string,
): Promise<ResponseInfo> {
  return new Promise((resolve, reject) => {
    const onResponse = async (response: HTTPResponse) => {
      const request = response.request();
      if (
        request.method() !== method || new URL(response.url()).pathname !== path
      ) return;
      clearTimeout(timer);
      page.off("response", onResponse);
      let body: string;
      try {
        body = await response.text();
      } catch (error) {
        body = `<response body unavailable: ${String(error)}>`;
      }
      resolve({
        status: response.status(),
        body,
        requestBody: request.postData(),
      });
    };
    const timer = setTimeout(() => {
      page.off("response", onResponse);
      reject(new Error(`Timed out waiting for ${method} ${path}`));
    }, WAIT);
    page.on("response", onResponse);
  });
}

function assertResponse(
  result: ResponseInfo,
  status: number,
  context: string,
): void {
  assertEquals(
    result.status,
    status,
    `${context}: status=${result.status} request=${
      result.requestBody ?? ""
    } body=${result.body}`,
  );
}

async function request(
  page: Page,
  path: string,
  method = "GET",
  body?: string,
  contentType?: string,
): Promise<ResponseInfo> {
  return await page.evaluate(
    async ({ path, method, body, contentType }) => {
      const headers: Record<string, string> = {
        Accept: "text/event-stream, text/html, application/json",
        "Datastar-Request": "true",
      };
      if (contentType) headers["Content-Type"] = contentType;
      const response = await fetch(path, { method, headers, body });
      return { status: response.status, body: await response.text() };
    },
    { path, method, body, contentType },
  );
}

async function items(page: Page, kind: Kind): Promise<Item[]> {
  const result = await request(page, `/editor/${editor[kind].path}`);
  assertResponse(result, 200, `GET ${kind}s`);
  const parsed: unknown = JSON.parse(result.body);
  assert(
    Array.isArray(parsed),
    `GET ${kind}s returned non-array body=${result.body}`,
  );
  return parsed as Item[];
}

function itemId(item: Item): number {
  const id = Number(item.id);
  assert(
    Number.isInteger(id) && id > 0,
    `Invalid fixture id: ${JSON.stringify(item)}`,
  );
  return id;
}

async function createFixture(
  page: Page,
  kind: Kind,
  title: string,
): Promise<number> {
  const body = kind === "quest"
    ? JSON.stringify({
      questTitle: title,
      questDescription: "API fixture description",
      questExpValue: 23,
      questDayOfWeek: 3,
      questImage: [],
    })
    : JSON.stringify({
      rewardTitle: title,
      rewardDescription: "API fixture description",
      rewardRequiredExp: 77,
      rewardImage: [],
    });
  const result = await request(
    page,
    `/editor/${editor[kind].path}`,
    "POST",
    body,
    "application/json",
  );
  assertResponse(result, 200, `create ${kind} fixture`);
  assert(result.body.length > 0, `create ${kind} fixture returned empty body`);
  const item = (await items(page, kind)).find((candidate) =>
    candidate.title === title
  );
  assert(item, `Created ${kind} fixture missing: ${title}`);
  return itemId(item);
}

async function deleteFixture(
  page: Page,
  kind: Kind,
  id: number,
): Promise<void> {
  const result = await request(
    page,
    `/editor/${editor[kind].path}/${id}`,
    "DELETE",
  );
  if (result.status !== 200 && result.status !== 404) {
    throw new Error(
      `cleanup ${kind} ${id}: status=${result.status} body=${result.body}`,
    );
  }
}

async function deleteByTitle(
  page: Page,
  kind: Kind,
  title: string,
): Promise<void> {
  for (const item of await items(page, kind)) {
    if (item.title === title) await deleteFixture(page, kind, itemId(item));
  }
}

async function login(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/editor`, {
    waitUntil: "domcontentloaded",
    timeout: WAIT,
  });
  await page.waitForSelector("#auth-modal", { visible: true, timeout: WAIT });
  await fill(page, "#password", "dev");
  await page.click(".auth-submit");
  await page.waitForSelector(".editor-container", {
    visible: true,
    timeout: WAIT,
  });
}

async function openTab(page: Page, kind: Kind): Promise<void> {
  await clickText(
    page,
    ".editor-tabs button",
    kind === "quest" ? "Quests" : "Rewards",
  );
  await page.waitForSelector(editor[kind].panel, {
    visible: true,
    timeout: WAIT,
  });
}

async function makeImage(): Promise<string> {
  const path = await Deno.makeTempFile({
    prefix: "quest-log-e2e-",
    suffix: ".png",
  });
  await Deno.writeFile(
    path,
    Uint8Array.from([
      137,
      80,
      78,
      71,
      13,
      10,
      26,
      10,
      0,
      0,
      0,
      13,
      73,
      72,
      68,
      82,
      0,
      0,
      0,
      1,
      0,
      0,
      0,
      1,
      8,
      6,
      0,
      0,
      0,
      31,
      21,
      196,
      137,
      0,
      0,
      0,
      13,
      73,
      68,
      65,
      84,
      120,
      156,
      99,
      96,
      0,
      0,
      0,
      2,
      0,
      1,
      226,
      33,
      188,
      51,
      0,
      0,
      0,
      0,
      73,
      69,
      78,
      68,
      174,
      66,
      96,
      130,
    ]),
  );
  return path;
}

function browserTest(
  name: string,
  fn: (page: Page) => Promise<void>,
  expectedStatuses: number[] = [],
): void {
  Deno.test({
    name,
    sanitizeResources: false,
    sanitizeOps: false,
    async fn() {
      const browser = await launchBrowser();
      try {
        const page = await newPage(browser);
        const errors = watchBrowserErrors(page);
        await fn(page);
        assertNoBrowserErrors(errors, expectedStatuses);
      } finally {
        await browser.close();
      }
    },
  });
}

for (const kind of ["quest", "reward"] as const) {
  for (const withImage of [false, true]) {
    browserTest(
      `${kind} create UI persists ${withImage ? "tiny image" : "text fields"}`,
      async (page) => {
        await login(page);
        await openTab(page, kind);
        const title = `qa-e2e-${kind}-${crypto.randomUUID()}`;
        const imagePath = await makeImage();
        let id: number | undefined;
        try {
          await page.click(`${editor[kind].panel} .panel-header button`);
          await page.waitForSelector(editor[kind].title, {
            visible: true,
            timeout: WAIT,
          });
          await fill(page, editor[kind].title, title);
          await fill(page, editor[kind].description, "UI image description");
          await fill(page, editor[kind].exp, kind === "quest" ? "31" : "91");
          if (kind === "quest") await page.select("#quest-day", "5");
          if (withImage) {
            const image = await page.$(editor[kind].image);
            assert(image, `Missing ${kind} image input`);
            const input = await image.toElement("input");
            await input.uploadFile(imagePath);
            await page.waitForFunction(
              (selector) =>
                (document.querySelector(selector) as HTMLInputElement)?.files
                  ?.length === 1,
              { timeout: WAIT },
              editor[kind].image,
            );
            await waitForImagePayloadReady(page, kind);
          }

          const response = responseFor(
            page,
            "POST",
            `/editor/${editor[kind].path}`,
          );
          await clickText(
            page,
            `${editor[kind].panel} .form-actions button`,
            "Save",
          );
          const result = await response;
          assertResponse(result, 200, `${kind} UI create`);
          assert(
            result.body.includes(title),
            `${kind} UI create body lacks title: ${result.body}`,
          );
          if (withImage) {
            assert(
              result.requestBody?.includes('"mime":"image/png"') === true &&
                result.requestBody.includes('"contents":"'),
              `${kind} UI create request lacks decoded image payload: ${result.requestBody}`,
            );
          }
          await waitForRow(page, kind, title, true);

          const item = (await items(page, kind)).find((candidate) =>
            candidate.title === title
          );
          assert(item, `UI-created ${kind} missing after reload inspection`);
          id = itemId(item);
          assertEquals(item.title, title);
          assertEquals(item.description, "UI image description");
          if (withImage) {
            assertEquals(item.image_content_type, "image/png");
            assert(
              Array.isArray(item.image_data) && item.image_data.length > 0,
            );
          }
          await page.reload({ waitUntil: "domcontentloaded", timeout: WAIT });
          await waitForRow(page, kind, title, true);
        } finally {
          await Deno.remove(imagePath).catch(() => {});
          if (id !== undefined) await deleteFixture(page, kind, id);
          else await deleteByTitle(page, kind, title);
        }
      },
    );
  }
}

for (const kind of ["quest", "reward"] as const) {
  browserTest(
    `${kind} edit populates fields and persists update`,
    async (page) => {
      await login(page);
      await openTab(page, kind);
      const title = `qa-e2e-edit-${kind}-${crypto.randomUUID()}`;
      const updatedTitle = `${title}-updated`;
      let id: number | undefined;
      try {
        const fixtureId = await createFixture(page, kind, title);
        id = fixtureId;
        await page.reload({ waitUntil: "domcontentloaded", timeout: WAIT });
        await page.waitForSelector(".editor-container", {
          visible: true,
          timeout: WAIT,
        });
        await openTab(page, kind);
        await waitForRow(page, kind, title, true);
        const population = responseFor(
          page,
          "GET",
          `/editor/${editor[kind].path}/${fixtureId}/edit`,
        );
        await clickRowAction(page, kind, title, "Edit");
        const populationResult = await population;
        assertResponse(populationResult, 200, `${kind} edit population`);
        assert(
          populationResult.body.includes(title),
          `edit population body lacks title: ${populationResult.body}`,
        );
        await page.waitForFunction(
          (
            {
              titleSelector,
              title,
              descriptionSelector,
              description,
              expSelector,
              exp,
            },
          ) =>
            (document.querySelector(titleSelector) as HTMLInputElement)
                ?.value === title &&
            (document.querySelector(descriptionSelector) as HTMLTextAreaElement)
                ?.value === description &&
            (document.querySelector(expSelector) as HTMLInputElement)?.value ===
              exp,
          { timeout: WAIT },
          {
            titleSelector: editor[kind].title,
            title,
            descriptionSelector: editor[kind].description,
            description: "API fixture description",
            expSelector: editor[kind].exp,
            exp: kind === "quest" ? "23" : "77",
          },
        );
        if (kind === "quest") {
          await page.waitForFunction(
            () =>
              (document.querySelector("#quest-day") as HTMLSelectElement)
                ?.value === "3",
            { timeout: WAIT },
          );
        }
        await fill(page, editor[kind].title, updatedTitle);
        await fill(page, editor[kind].description, "Updated through editor");
        await fill(page, editor[kind].exp, kind === "quest" ? "37" : "97");
        if (kind === "quest") await page.select("#quest-day", "6");
        const update = responseFor(
          page,
          "PUT",
          `/editor/${editor[kind].path}/${fixtureId}`,
        );
        await clickText(
          page,
          `${editor[kind].panel} .form-actions button`,
          "Update",
        );
        const updateResult = await update;
        assertResponse(updateResult, 200, `${kind} UI update`);
        assert(
          updateResult.body.includes(updatedTitle),
          `update body lacks title: ${updateResult.body}`,
        );
        await waitForRow(page, kind, updatedTitle, true);
        const stored = (await items(page, kind)).find((item) =>
          item.id === fixtureId
        );
        assert(stored, `updated ${kind} missing from API`);
        assertEquals(stored.title, updatedTitle);
        assertEquals(stored.description, "Updated through editor");
        assertEquals(
          stored[kind === "quest" ? "exp_value" : "required_exp"],
          kind === "quest" ? 37 : 97,
        );
        if (kind === "quest") assertEquals(stored.day_of_week, 6);
      } finally {
        if (id === undefined) await deleteByTitle(page, kind, title);
        else await deleteFixture(page, kind, id);
      }
    },
  );
}

for (const kind of ["quest", "reward"] as const) {
  browserTest(
    `${kind} delete removes DOM row and survives reload`,
    async (page) => {
      await login(page);
      await openTab(page, kind);
      const title = `qa-e2e-delete-${kind}-${crypto.randomUUID()}`;
      let id: number | undefined;
      let needsCleanup = true;
      try {
        const fixtureId = await createFixture(page, kind, title);
        id = fixtureId;
        await page.reload({ waitUntil: "domcontentloaded", timeout: WAIT });
        await page.waitForSelector(".editor-container", {
          visible: true,
          timeout: WAIT,
        });
        await openTab(page, kind);
        await waitForRow(page, kind, title, true);
        const deletion = responseFor(
          page,
          "DELETE",
          `/editor/${editor[kind].path}/${fixtureId}`,
        );
        await clickRowAction(page, kind, title, "Delete");
        const deletionResult = await deletion;
        assertResponse(deletionResult, 200, `${kind} UI delete`);
        assert(
          deletionResult.body.length > 0,
          `${kind} delete returned empty body`,
        );
        await waitForRow(page, kind, title, false);
        await page.reload({ waitUntil: "domcontentloaded", timeout: WAIT });
        await page.waitForSelector(".editor-container", {
          visible: true,
          timeout: WAIT,
        });
        await openTab(page, kind);
        await waitForRow(page, kind, title, false);
        assert(
          !(await items(page, kind)).some((item) => item.id === fixtureId),
        );
        needsCleanup = false;
      } finally {
        if (needsCleanup) {
          if (id === undefined) await deleteByTitle(page, kind, title);
          else await deleteFixture(page, kind, id);
        }
      }
    },
  );
}

browserTest(
  "settings change persists, restores, and survives reload",
  async (page) => {
    await login(page);
    const settingsResult = await request(page, "/editor/settings");
    assertResponse(settingsResult, 200, "GET settings");
    const initial = Number(
      (JSON.parse(settingsResult.body) as Item).weekly_exp_goal,
    );
    const changed = initial + 1;
    try {
      await openTab(page, "quest");
      await clickText(page, ".editor-tabs button", "Settings");
      await page.waitForSelector("#settings-panel", {
        visible: true,
        timeout: WAIT,
      });
      await fill(page, "#weekly-exp-goal", String(changed));
      const save = responseFor(page, "PUT", "/editor/settings");
      await page.click("#settings-form button[type=submit]");
      const saveResult = await save;
      assertResponse(saveResult, 200, "save changed settings");
      await page.reload({ waitUntil: "domcontentloaded", timeout: WAIT });
      await page.waitForSelector(".editor-container", {
        visible: true,
        timeout: WAIT,
      });
      await clickText(page, ".editor-tabs button", "Settings");
      await page.waitForSelector("#settings-panel", {
        visible: true,
        timeout: WAIT,
      });
      await page.waitForFunction(
        (value) =>
          (document.querySelector("#weekly-exp-goal") as HTMLInputElement)
            ?.value === value,
        { timeout: WAIT },
        String(changed),
      );
      await fill(page, "#weekly-exp-goal", String(initial));
      const restore = responseFor(page, "PUT", "/editor/settings");
      await page.click("#settings-form button[type=submit]");
      const restoreResult = await restore;
      assertResponse(restoreResult, 200, "restore settings");
      await page.reload({ waitUntil: "domcontentloaded", timeout: WAIT });
      await page.waitForSelector(".editor-container", {
        visible: true,
        timeout: WAIT,
      });
      await clickText(page, ".editor-tabs button", "Settings");
      await page.waitForSelector("#settings-panel", {
        visible: true,
        timeout: WAIT,
      });
      await page.waitForFunction(
        (value) =>
          (document.querySelector("#weekly-exp-goal") as HTMLInputElement)
            ?.value === value,
        { timeout: WAIT },
        String(initial),
      );
    } finally {
      const result = await request(
        page,
        "/editor/settings",
        "PUT",
        new URLSearchParams({ weekly_exp_goal: String(initial) }).toString(),
        "application/x-www-form-urlencoded",
      );
      assertResponse(result, 200, "finally restore settings");
    }
  },
);

browserTest("invalid editor login exposes an error", async (page) => {
  await page.goto(`${BASE_URL}/editor`, {
    waitUntil: "domcontentloaded",
    timeout: WAIT,
  });
  await page.waitForSelector("#auth-modal", { visible: true, timeout: WAIT });
  await fill(page, "#password", "not-the-master-key");
  const response = responseFor(page, "POST", "/editor/login");
  await page.click(".auth-submit");
  const result = await response;
  assertResponse(result, 200, "invalid login");
  assert(
    result.body.includes("Invalid password"),
    `invalid login body: ${result.body}`,
  );
  await page.waitForFunction(
    () =>
      document.querySelector(".auth-error")?.textContent?.includes(
        "Invalid password",
      ) === true,
    { timeout: WAIT },
  );
});

browserTest("logout removes session and protects editor API", async (page) => {
  await login(page);
  const logout = responseFor(page, "POST", "/editor/logout");
  await page.click(".logout-btn");
  const logoutResult = await logout;
  assertResponse(logoutResult, 200, "logout");
  await page.waitForSelector("#auth-modal", { visible: true, timeout: WAIT });
  await page.reload({ waitUntil: "domcontentloaded", timeout: WAIT });
  await page.waitForSelector("#auth-modal", { visible: true, timeout: WAIT });
  const protectedResult = await request(page, "/editor/quests");
  assertResponse(protectedResult, 401, "protected quests after logout");
  assert(
    protectedResult.body.includes("Access Denied"),
    `protected body: ${protectedResult.body}`,
  );
}, [401]);
