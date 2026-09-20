import { assert, assertEquals } from "@std/assert";
import type { Page } from "puppeteer-core";
import { launchBrowser, newPage, typeIntoInput } from "./browser.ts";

type BrowserResponse = {
  status: number;
  contentType: string;
  body: string;
};
type Row = Record<string, unknown>;

type Fixture = { id: number };

const BASE_URL = "http://localhost:3000";
const WAIT = { timeout: 5_000 };

async function login(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/editor`);
  await page.waitForSelector("#auth-modal", { visible: true, ...WAIT });
  await typeIntoInput(page, "#password", "dev");
  await page.click(".auth-submit");
  await page.waitForSelector(".editor-container", { visible: true, ...WAIT });
}

async function markIntroSeen(page: Page): Promise<void> {
  await page.evaluate(() =>
    localStorage.setItem("quest-log-first-time", "true")
  );
}

async function browserFetch(
  page: Page,
  path: string,
  method = "GET",
  body?: unknown,
): Promise<BrowserResponse> {
  const bodyText = body === undefined ? undefined : JSON.stringify(body);
  return await page.evaluate(
    async ({ path, method, bodyText }) => {
      const response = await fetch(path, {
        method,
        headers: {
          Accept: "text/event-stream, application/json",
          "Content-Type": "application/json",
          "Datastar-Request": "true",
        },
        body: bodyText,
      });
      return {
        status: response.status,
        contentType: response.headers.get("content-type") ?? "",
        body: await response.text(),
      };
    },
    { path, method, bodyText },
  );
}

function assertSse(response: BrowserResponse, label: string): void {
  assertEquals(response.status, 200, `${label}: ${response.body}`);
  assert(
    response.contentType.includes("text/event-stream"),
    `${label}: ${response.contentType}\n${response.body}`,
  );
  assert(response.body.includes("datastar"), `${label}: ${response.body}`);
}

function rows(response: BrowserResponse, label: string): Row[] {
  assertEquals(response.status, 200, `${label}: ${response.body}`);
  assert(
    response.contentType.includes("application/json"),
    `${label}: ${response.contentType}\n${response.body}`,
  );
  const value: unknown = JSON.parse(response.body);
  assert(
    Array.isArray(value),
    `${label}: expected JSON array: ${response.body}`,
  );
  return value as Row[];
}

function rowId(row: Row, label: string): number {
  assert(typeof row.id === "number", `${label}: missing numeric id`);
  return row.id as number;
}

function uniqueTitle(prefix: string): string {
  return `QA gameplay ${prefix} ${crypto.randomUUID()}`;
}

function todayFromBrowser(page: Page): Promise<{ iso: string; day: number }> {
  const override = Deno.env.get("QUEST_LOG_TODAY")?.trim() ?? "";
  if (/^\d{4}-\d{2}-\d{2}$/.test(override)) {
    const [year, month, day] = override.split("-").map(Number);
    return Promise.resolve({
      iso: override,
      day: new Date(Date.UTC(year, month - 1, day)).getUTCDay(),
    });
  }
  return page.evaluate(() => {
    const now = new Date();
    const pad = (value: number) => String(value).padStart(2, "0");
    return {
      iso: `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${
        pad(now.getDate())
      }`,
      day: now.getDay(),
    };
  });
}

async function createFixture(
  page: Page,
  path: "quests" | "rewards",
  title: string,
  payload: Record<string, unknown>,
): Promise<Fixture> {
  const response = await browserFetch(page, `/editor/${path}`, "POST", payload);
  assertSse(response, `create ${path}`);
  assert(
    response.body.includes(title),
    `create ${path} payload: ${response.body}`,
  );
  const found = rows(
    await browserFetch(page, `/editor/${path}`),
    `list ${path} after create`,
  ).find((row) => row.title === title);
  assert(found, `created ${path} missing: ${title}`);
  return { id: rowId(found, `created ${path}`) };
}

function createQuest(
  page: Page,
  title: string,
  day: number,
  exp: number,
): Promise<Fixture> {
  return createFixture(page, "quests", title, {
    questTitle: title,
    questDescription: "owned gameplay fixture",
    questExpValue: exp,
    questDayOfWeek: day,
    questImage: [],
  });
}

function createReward(page: Page, title: string): Promise<Fixture> {
  return createFixture(page, "rewards", title, {
    rewardTitle: title,
    rewardDescription: "owned gameplay fixture",
    rewardRequiredExp: 0,
    rewardImage: [],
  });
}

async function cleanup(
  page: Page,
  path: "quests" | "rewards",
  title: string,
  id?: number,
): Promise<void> {
  const base = `/editor/${path}`;
  const ids = id === undefined
    ? rows(await browserFetch(page, base), `cleanup ${path}`)
      .filter((row) => row.title === title)
      .map((row) => rowId(row, `cleanup ${path}`))
    : [id];
  for (const ownId of ids) {
    await browserFetch(page, `${base}/${ownId}`, "DELETE");
  }
}

async function questStats(
  page: Page,
): Promise<{ exp: number; completed: number }> {
  return await page.evaluate(() => {
    const expText = document.querySelector("#exp-counter")?.textContent ?? "";
    const completedText =
      [...document.querySelectorAll("#stats-panel .stat-row")]
        .find((row) => row.textContent?.includes("Quests Completed"))
        ?.textContent ?? "";
    const number = (text: string) => Number(text.match(/-?\d+/)?.[0] ?? NaN);
    return { exp: number(expText), completed: number(completedText) };
  });
}

function assertToggleRequest(
  response: Awaited<ReturnType<Page["waitForResponse"]>>,
  id: number,
  label: string,
): void {
  const request = response.request();
  assertEquals(request.method(), "POST", `${label}: method`);
  const headers = request.headers();
  assert(
    (headers["content-type"] ?? "").includes("application/json"),
    `${label}: headers ${JSON.stringify(headers)}`,
  );
  assert(
    (headers["datastar-request"] ?? "").toLowerCase() === "true",
    `${label}: headers ${JSON.stringify(headers)}`,
  );
  assert(
    (request.postData() ?? "").includes(`"quest_id":${id}`),
    `${label}: ${request.postData()}`,
  );
}

async function clickQuest(page: Page, id: number): Promise<BrowserResponse> {
  await page.bringToFront();
  const [response] = await Promise.all([
    page.waitForResponse(
      (response) =>
        response.url().endsWith("/quests/toggle") &&
        response.request().method() === "POST",
      WAIT,
    ),
    page.click(`#quest-${id} .toggle-btn`),
  ]);
  const body = await response.text();
  const result = {
    status: response.status(),
    contentType: response.headers()["content-type"] ?? "",
    body,
  };
  assertToggleRequest(response, id, "toggle quest");
  assertSse(result, "toggle quest");
  return result;
}

function displayDate(iso: string): string {
  const [year, month, day] = iso.split("-").map(Number);
  const months = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  ];
  return `${day} ${months[month - 1]} ${year}`;
}

async function highscore(page: Page): Promise<{
  totalExp: number;
  questsCompleted: number;
  history: string[];
}> {
  await page.goto(`${BASE_URL}/highscore`);
  await page.waitForSelector("h1", WAIT);
  return await page.evaluate(() => {
    const cards = [...document.querySelectorAll(".stat-card")].map((card) => ({
      label: card.querySelector(".stat-label")?.textContent?.trim() ?? "",
      value: Number(
        card.querySelector(".stat-value")?.textContent?.match(/-?\d+/)?.[0] ??
          NaN,
      ),
    }));
    const value = (label: string) =>
      cards.find((card) => card.label === label)?.value ?? NaN;
    const history = [...document.querySelectorAll(".history-table tbody tr")]
      .map((row) =>
        [...row.querySelectorAll("td")].map((cell) =>
          cell.textContent?.trim() ?? ""
        ).join(" ")
      );
    return {
      totalExp: value("Total EXP"),
      questsCompleted: value("Quests Completed"),
      history,
    };
  });
}

function historyCount(history: string[], date: string): number {
  const row = history.find((value) => value.startsWith(`${date} `));
  return row === undefined ? 0 : Number(row.match(/(\d+)$/)?.[1] ?? NaN);
}

Deno.test({
  name: "Gameplay direct completion, reopen, stats, reload, highscore history",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    const page = await newPage(browser);
    const title = uniqueTitle("completion");
    let fixtureId: number | undefined;
    let loggedIn = false;
    try {
      await login(page);
      loggedIn = true;
      const today = await todayFromBrowser(page);
      const fixture = await createQuest(page, title, today.day, 7);
      fixtureId = fixture.id;
      await markIntroSeen(page);
      await page.goto(BASE_URL);
      await page.waitForSelector(`#quest-${fixture.id} .toggle-btn`, WAIT);

      const before = await questStats(page);
      const baseHighscore = await highscore(page);
      const baseDateCount = historyCount(
        baseHighscore.history,
        displayDate(today.iso),
      );

      await page.goto(BASE_URL);
      await page.waitForSelector(`#quest-${fixture.id} .toggle-btn`, WAIT);
      await clickQuest(page, fixture.id);
      await page.waitForSelector(`#quest-${fixture.id}.completed`, WAIT);
      await page.waitForFunction(
        (expected) =>
          document.querySelector("#exp-counter")?.textContent?.includes(
            String(expected),
          ),
        WAIT,
        before.exp + 7,
      );
      const completedStats = await questStats(page);
      assertEquals(completedStats.exp, before.exp + 7);
      assertEquals(completedStats.completed, before.completed + 1);

      const completedHighscore = await highscore(page);
      assertEquals(completedHighscore.totalExp, baseHighscore.totalExp + 7);
      assertEquals(
        completedHighscore.questsCompleted,
        baseHighscore.questsCompleted + 1,
      );
      assertEquals(
        historyCount(completedHighscore.history, displayDate(today.iso)),
        baseDateCount + 1,
      );

      await page.goto(BASE_URL);
      await page.waitForSelector(
        `#quest-${fixture.id}.completed .toggle-btn`,
        WAIT,
      );
      await clickQuest(page, fixture.id);
      await page.waitForFunction(
        (selector) =>
          document.querySelector(selector)?.classList.contains("completed") ===
            false,
        WAIT,
        `#quest-${fixture.id}`,
      );
      await page.reload();
      await page.waitForSelector(`#quest-${fixture.id} .toggle-btn`, WAIT);
      assertEquals(await questStats(page), before);

      const reopenedHighscore = await highscore(page);
      assertEquals(reopenedHighscore.totalExp, baseHighscore.totalExp);
      assertEquals(
        reopenedHighscore.questsCompleted,
        baseHighscore.questsCompleted,
      );
      assertEquals(
        historyCount(reopenedHighscore.history, displayDate(today.iso)),
        baseDateCount,
      );
    } finally {
      try {
        if (loggedIn) await cleanup(page, "quests", title, fixtureId);
      } finally {
        await browser.close();
      }
    }
  },
});

Deno.test({
  name: "Gameplay observer tab receives quest completion through SSE",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    const setupPage = await newPage(browser);
    const title = uniqueTitle("sse");
    let fixtureId: number | undefined;
    let loggedIn = false;
    try {
      await login(setupPage);
      loggedIn = true;
      const today = await todayFromBrowser(setupPage);
      const fixture = await createQuest(setupPage, title, today.day, 5);
      fixtureId = fixture.id;

      const actor = await newPage(browser);
      const observer = await newPage(browser);
      await markIntroSeen(actor);
      await markIntroSeen(observer);
      let observerToggleRequests = 0;
      observer.on("request", (request) => {
        if (request.url().endsWith("/quests/toggle")) {
          observerToggleRequests += 1;
        }
      });
      await actor.goto(BASE_URL);
      await Promise.all([
        observer.waitForRequest(
          (request) => request.url().endsWith("/events"),
          WAIT,
        ),
        observer.goto(BASE_URL),
      ]);
      await actor.waitForSelector(`#quest-${fixture.id} .toggle-btn`, WAIT);
      await observer.waitForSelector(`#quest-${fixture.id} .toggle-btn`, WAIT);
      const observerBefore = await questStats(observer);

      await clickQuest(actor, fixture.id);
      await observer.waitForSelector(`#quest-${fixture.id}.completed`, WAIT);
      await observer.waitForFunction(
        (expected) =>
          document.querySelector("#exp-counter")?.textContent?.includes(
            String(expected),
          ),
        WAIT,
        observerBefore.exp + 5,
      );
      assertEquals(observerToggleRequests, 0);
      assertEquals(await questStats(observer), {
        exp: observerBefore.exp + 5,
        completed: observerBefore.completed + 1,
      });

      await actor.close();
      await observer.close();
    } finally {
      try {
        if (loggedIn) await cleanup(setupPage, "quests", title, fixtureId);
      } finally {
        await browser.close();
      }
    }
  },
});

Deno.test({
  name: "Gameplay bounty claims own zero-EXP reward once and persists",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    const page = await newPage(browser);
    const title = uniqueTitle("reward");
    let fixtureId: number | undefined;
    let loggedIn = false;
    try {
      await login(page);
      loggedIn = true;
      const today = await todayFromBrowser(page);
      const fixture = await createReward(page, title);
      fixtureId = fixture.id;
      await markIntroSeen(page);
      await page.goto(`${BASE_URL}/bounty`);
      await page.waitForSelector(`#reward-${fixture.id} .claim-btn`, WAIT);

      const button = await page.$eval(
        `#reward-${fixture.id} .claim-btn`,
        (element) => ({
          disabled: (element as HTMLButtonElement).disabled,
          text: element.textContent?.trim() ?? "",
          className: element.className,
        }),
      );
      if (today.day !== 0) {
        assertEquals(button.disabled, true);
        assert(button.text.includes("Available Sunday"), button.text);
        const rejected = await browserFetch(page, "/rewards/claim", "POST", {
          reward_id: fixture.id,
        });
        assertEquals(rejected.status, 400, rejected.body);
        assert(rejected.body.includes("non-Sunday"), rejected.body);
      } else {
        assertEquals(button.disabled, false);
        assert(button.className.includes("claimable"), button.className);
        const [response] = await Promise.all([
          page.waitForResponse(
            (response) =>
              response.url().endsWith("/rewards/claim") &&
              response.request().method() === "POST",
            WAIT,
          ),
          page.click(`#reward-${fixture.id} .claim-btn`),
        ]);
        const body = await response.text();
        const result = {
          status: response.status(),
          contentType: response.headers()["content-type"] ?? "",
          body,
        };
        assertEquals(response.request().method(), "POST");
        assert(
          (response.request().postData() ?? "").includes(
            `"reward_id":${fixture.id}`,
          ),
        );
        assertSse(result, "claim reward");
        await page.waitForSelector(
          `#reward-${fixture.id} .claim-btn.claimed`,
          WAIT,
        );

        const duplicate = await browserFetch(page, "/rewards/claim", "POST", {
          reward_id: fixture.id,
        });
        assertEquals(duplicate.status, 400, duplicate.body);
        assert(duplicate.body.includes("already claimed"), duplicate.body);

        await page.reload();
        await page.waitForSelector(
          `#reward-${fixture.id} .claim-btn.claimed`,
          WAIT,
        );
        const persisted = await page.$eval(
          `#reward-${fixture.id} .claim-btn`,
          (element) => ({
            disabled: (element as HTMLButtonElement).disabled,
            text: element.textContent?.trim() ?? "",
          }),
        );
        assertEquals(persisted.disabled, true);
        assert(persisted.text.includes("CLAIMED"), persisted.text);
      }
    } finally {
      try {
        if (loggedIn) await cleanup(page, "rewards", title, fixtureId);
      } finally {
        await browser.close();
      }
    }
  },
});
