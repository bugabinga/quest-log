import { assert, assertEquals } from "@std/assert";
import type { HTTPRequest, Page } from "puppeteer-core";
import { launchBrowser } from "./browser.ts";

const BASE_URL = "http://localhost:3000";
const SCREENSHOT_DIR = "target/quest-log/qa/screenshots";
const WAIT = 5000;

type Failure = {
  url: string;
  status: number;
  body: Promise<string>;
};

type Runtime = {
  errors: string[];
  failures: Failure[];
  diagnostics: () => Promise<{ errors: string[]; failures: FailureReport[] }>;
};

type FailureReport = Omit<Failure, "body"> & { body: string };

function watchRuntime(page: Page): Runtime {
  const errors: string[] = [];
  const failures: Failure[] = [];

  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("response", (response) => {
    if (!sameOrigin(response.url()) || response.status() < 400) return;
    failures.push({
      url: response.url(),
      status: response.status(),
      body: response.text().catch((error) => String(error)),
    });
  });
  page.on("requestfailed", (request) => {
    if (!sameOrigin(request.url())) return;
    if (
      ["eventsource", "media"].includes(request.resourceType()) &&
      request.failure()?.errorText === "net::ERR_ABORTED"
    ) return;
    failures.push({
      url: request.url(),
      status: 0,
      body: Promise.resolve(request.failure()?.errorText ?? "request failed"),
    });
  });

  return {
    errors,
    failures,
    async diagnostics() {
      const reports = await Promise.all(
        failures.map(async ({ body, ...failure }) => ({
          ...failure,
          body: await body,
        })),
      );
      return { errors: [...errors], failures: reports };
    },
  };
}

function sameOrigin(url: string): boolean {
  return new URL(url).origin === BASE_URL;
}

async function setTimezoneCookie(page: Page): Promise<void> {
  const timezone = await page.evaluate(() =>
    Intl.DateTimeFormat().resolvedOptions().timeZone
  );
  await page.setCookie({
    name: "QuestLog-TZ",
    value: timezone,
    url: BASE_URL,
  });
}

function waitForEventSource(page: Page): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      page.off("request", onRequest);
      reject(
        new Error("Quest Log did not subscribe to /events within 5 seconds"),
      );
    }, WAIT);
    function onRequest(request: HTTPRequest) {
      if (
        sameOrigin(request.url()) &&
        request.url().endsWith("/events") &&
        request.resourceType() === "eventsource"
      ) {
        clearTimeout(timer);
        page.off("request", onRequest);
        resolve();
      }
    }
    page.on("request", onRequest);
  });
}

async function preparePage(page: Page, path: string): Promise<Runtime> {
  await setTimezoneCookie(page);
  await page.evaluateOnNewDocument(() =>
    localStorage.setItem("quest-log-first-time", "true")
  );
  if (page.url().startsWith(BASE_URL)) {
    await page.evaluate(() =>
      localStorage.setItem("quest-log-first-time", "true")
    );
  }
  const runtime = watchRuntime(page);
  await page.goto(`${BASE_URL}${path}`, {
    waitUntil: "domcontentloaded",
    timeout: WAIT,
  });
  return runtime;
}

async function waitForDay(page: Page, date: string): Promise<void> {
  try {
    await page.waitForFunction(
      (expected: string) =>
        document.querySelector(".day-date")?.textContent?.trim() === expected,
      { timeout: WAIT },
      date,
    );
    await page.waitForFunction(
      () => !document.documentElement.matches(":active-view-transition"),
      { timeout: WAIT },
    );
  } catch (error) {
    const actual = await page.evaluate(() => ({
      url: location.href,
      date: document.querySelector(".day-date")?.textContent,
      title: document.title,
    }));
    throw new Error(
      `Expected date ${date}; actual ${JSON.stringify(actual)}; ${error}`,
    );
  }
}

async function waitForModal(
  page: Page,
  display: "flex" | "none",
): Promise<void> {
  await page.waitForFunction(
    (expected: string) => {
      const modal = document.querySelector<HTMLElement>("#video-modal");
      return modal !== null && getComputedStyle(modal).display === expected;
    },
    { timeout: WAIT },
    display,
  );
}

async function waitForVideoPlayback(page: Page, muted: boolean): Promise<void> {
  await page.waitForFunction(
    (expectedMuted: boolean) => {
      const video = document.querySelector<HTMLVideoElement>("#intro-video");
      return video !== null && !video.paused && video.currentTime > 0 &&
        video.muted === expectedMuted;
    },
    { timeout: WAIT },
    muted,
  );
}

async function assertClean(runtime: Runtime, label: string): Promise<void> {
  const diagnostics = await runtime.diagnostics();
  assertEquals(
    diagnostics.errors,
    [],
    `${label}: JavaScript errors: ${JSON.stringify(diagnostics.errors)}`,
  );
  assertEquals(
    diagnostics.failures,
    [],
    `${label}: failed same-origin responses: ${
      JSON.stringify(diagnostics.failures)
    }`,
  );
}

Deno.test({
  name: "public navigation pages render accessible controls and clean runtime",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      for (
        const route of [
          ["/", "Quest Log", "h1"],
          ["/bounty", "Bounty Board", "h1"],
          ["/highscore", "Highscore", "h1"],
          ["/editor", "Quest Log Editor", "#auth-modal"],
        ] as const
      ) {
        const page = await browser.newPage();
        try {
          const runtime = await preparePage(page, route[0]);
          await page.waitForSelector(route[2], {
            visible: true,
            timeout: WAIT,
          });
          const state = await page.$eval("body", (body) => ({
            text: body.textContent ?? "",
            navLinks: [...document.querySelectorAll("nav a")].map((link) => ({
              text: link.textContent?.trim() ?? "",
              visible: getComputedStyle(link).display !== "none" &&
                link.getBoundingClientRect().width > 0,
            })),
            controls: [...document.querySelectorAll("nav a, button")].map((
              el,
            ) => ({
              name: el.getAttribute("aria-label") ?? el.textContent?.trim() ??
                "",
              visible: getComputedStyle(el).display !== "none" &&
                el.getBoundingClientRect().width > 0,
            })),
            trailerVisible: (() => {
              const trailer = document.querySelector<HTMLElement>(
                ".nav-link--trailer",
              );
              return trailer !== null &&
                getComputedStyle(trailer).display !== "none" &&
                trailer.getBoundingClientRect().width > 0;
            })(),
          }));
          assert(state.text.includes(route[1]), `${route[0]} marker missing`);
          assertEquals(state.navLinks.length, 4, `${route[0]} nav links`);
          assert(
            state.navLinks.every((link) => link.text && link.visible),
            `${route[0]} nav links must be visible and named`,
          );
          assert(
            state.controls.filter((control) => control.visible).every((
              control,
            ) => control.name),
            `${route[0]} visible controls must be named`,
          );
          assert(
            state.trailerVisible,
            `${route[0]} trailer control must be visible`,
          );
          await page.waitForSelector("nav", { visible: true, timeout: WAIT });
          await assertClean(runtime, route[0]);
        } finally {
          await page.close();
        }
      }
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "runtime subscribes to server events",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await browser.newPage();
      await Promise.all([
        waitForEventSource(page),
        page.goto(BASE_URL, { waitUntil: "domcontentloaded", timeout: WAIT }),
      ]);
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "manifest icons font backgrounds and trailer assets load",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await browser.newPage();
      try {
        const trailerRequests: string[] = [];
        page.on("request", (request) => {
          if (request.url() === `${BASE_URL}/video/trailer.mp4`) {
            trailerRequests.push(request.url());
          }
        });
        const runtime = await preparePage(page, "/");
        await page.waitForSelector("h1", { visible: true, timeout: WAIT });
        await page.waitForFunction(
          () => document.readyState === "complete",
          { timeout: WAIT },
        );
        assertEquals(trailerRequests, []);
        const manifest = await page.evaluate(async () => {
          const response = await fetch("/manifest.json", {
            cache: "no-store",
            signal: AbortSignal.timeout(5000),
          });
          return { status: response.status, value: await response.json() };
        });
        assertEquals(manifest.status, 200);
        assertEquals(
          manifest.value.icons.map((icon: { src: string }) => icon.src),
          ["/images/icon-192.png", "/images/icon-512.png"],
        );
        const backgroundPaths = [
          "/images/sunday.webp",
          "/images/monday.webp",
          "/images/tuesday.webp",
          "/images/wednesday.webp",
          "/images/thursday.webp",
          "/images/friday.webp",
          "/images/saturday.webp",
        ];
        const assets = await page.evaluate(async (paths) => {
          return await Promise.all(paths.map(async (path) => {
            const response = await fetch(path, {
              cache: "no-store",
              signal: AbortSignal.timeout(5000),
            });
            return {
              path,
              status: response.status,
              bytes: (await response.arrayBuffer()).byteLength,
            };
          }));
        }, [
          "/favicon.png",
          "/images/icon-192.png",
          "/images/icon-512.png",
          "/fonts/PressStart2P.woff2",
          ...backgroundPaths,
          "/video/trailer.mp4",
        ]);
        assert(
          assets.every((asset) => asset.status === 200 && asset.bytes > 0),
          JSON.stringify(assets),
        );
        const backgroundBytes = assets
          .filter((asset) => backgroundPaths.includes(asset.path))
          .reduce((total, asset) => total + asset.bytes, 0);
        assert(
          backgroundBytes <= 65_536,
          `Background payload exceeds 64 KiB: ${backgroundBytes}`,
        );
        const trailerBytes = assets.find((asset) =>
          asset.path === "/video/trailer.mp4"
        )?.bytes ?? Infinity;
        assert(
          trailerBytes <= 3 * 1024 * 1024,
          `Trailer payload exceeds 3 MiB: ${trailerBytes}`,
        );
        const trailerHeader = await page.evaluate(async () => {
          const response = await fetch("/video/trailer.mp4", {
            headers: { Range: "bytes=0-4095" },
            cache: "no-store",
            signal: AbortSignal.timeout(5000),
          });
          const bytes = new Uint8Array(await response.arrayBuffer());
          const moov = [0x6d, 0x6f, 0x6f, 0x76];
          return {
            status: response.status,
            hasMoov: bytes.some((_, index) =>
              moov.every((byte, offset) => bytes[index + offset] === byte)
            ),
          };
        });
        assertEquals(trailerHeader, { status: 206, hasMoov: true });
        await assertClean(runtime, "assets");
      } finally {
        await page.close();
      }
    } finally {
      await browser.close();
    }
  },
});

for (const mode of ["buttons", "keyboard", "history"] as const) {
  Deno.test({
    name: `day navigation ${mode} stays consistent`,
    sanitizeResources: false,
    sanitizeOps: false,
    async fn() {
      const browser = await launchBrowser();
      try {
        const page = await browser.newPage();
        try {
          const runtime = await preparePage(page, "/");
          await page.waitForSelector(".day-date", {
            visible: true,
            timeout: WAIT,
          });
          const direction = await page.$eval(
            "#nav-left",
            (el) => el.classList.contains("disabled") ? "right" : "left",
          );
          const todayDate = await page.$eval(
            ".day-date",
            (el) => el.textContent?.trim() ?? "",
          );
          await page.click(`#nav-${direction}`);
          await page.waitForFunction(
            (date) =>
              document.querySelector(".day-date")?.textContent?.trim() !== date,
            { timeout: WAIT },
            todayDate,
          );
          const previousDate = await page.$eval(
            ".day-date",
            (el) => el.textContent?.trim() ?? "",
          );
          assert(new URL(page.url()).pathname.startsWith("/day/"));
          await page.waitForFunction(
            () => !document.documentElement.matches(":active-view-transition"),
            { timeout: WAIT },
          );
          if (mode === "buttons") {
            await page.click("#today-btn-container .today-btn");
            await waitForDay(page, todayDate);
            assertEquals(new URL(page.url()).pathname, "/");
          } else if (mode === "keyboard") {
            await page.keyboard.press(
              direction === "left" ? "ArrowRight" : "ArrowLeft",
            );
            await waitForDay(page, todayDate);
            await page.keyboard.press(
              direction === "left" ? "ArrowLeft" : "ArrowRight",
            );
            await waitForDay(page, previousDate);
            await page.keyboard.press("t");
            await waitForDay(page, todayDate);
            assertEquals(new URL(page.url()).pathname, "/");
          } else {
            await page.goBack({ waitUntil: "domcontentloaded", timeout: WAIT });
            await page.waitForFunction(
              () => new URL(location.href).pathname === "/",
              { timeout: WAIT },
            );
            await waitForDay(page, todayDate);
            await page.goForward({
              waitUntil: "domcontentloaded",
              timeout: WAIT,
            });
            await page.waitForFunction(
              () => new URL(location.href).pathname.startsWith("/day/"),
              { timeout: WAIT },
            );
            await waitForDay(page, previousDate);
          }
          await assertClean(runtime, "day navigation");
        } finally {
          await page.close();
        }
      } finally {
        await browser.close();
      }
    },
  });
}

Deno.test({
  name: "trailer autoplays muted then replays with sound from header",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await browser.newPage();
      try {
        await setTimezoneCookie(page);
        const runtime = watchRuntime(page);
        await page.goto(BASE_URL, {
          waitUntil: "domcontentloaded",
          timeout: WAIT,
        });

        await waitForModal(page, "flex");
        await waitForVideoPlayback(page, true);
        await page.click(".video-close");
        await waitForModal(page, "none");

        await page.click(".nav-link--trailer");
        await waitForModal(page, "flex");
        await waitForVideoPlayback(page, false);
        await page.keyboard.press("Escape");
        await waitForModal(page, "none");
        await assertClean(runtime, "trailer");
      } finally {
        await page.close();
      }
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "mobile and desktop dark layouts do not overflow",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    await Deno.mkdir(SCREENSHOT_DIR, { recursive: true });
    const problems: string[] = [];
    const browser = await launchBrowser();
    async function capture(page: Page, name: string): Promise<void> {
      await page.evaluate(() => document.fonts.ready.then(() => undefined));
      await page.waitForFunction(
        () =>
          document.getAnimations().every((animation) =>
            animation.effect?.getTiming().iterations === Infinity ||
            animation.playState !== "running"
          ),
        { timeout: WAIT },
      );
      const layout = await page.$eval("html", (html) => ({
        dark: getComputedStyle(html).colorScheme.includes("dark"),
        overflow: html.scrollWidth > html.clientWidth ||
          document.body.scrollWidth > document.body.clientWidth,
        clippedActions: [...document.querySelectorAll(".editor-table button")]
          .filter((button) => {
            const rect = button.getBoundingClientRect();
            if (!rect.width || (rect.left >= 0 && rect.right <= innerWidth)) {
              return false;
            }
            for (
              let parent = button.parentElement;
              parent;
              parent = parent.parentElement
            ) {
              if (
                ["auto", "scroll"].includes(
                  getComputedStyle(parent).overflowX,
                ) && parent.scrollWidth > parent.clientWidth
              ) return false;
            }
            return true;
          }).map((button) => button.textContent?.trim()),
      }));
      if (!layout.dark) problems.push(`${name}: not dark`);
      if (layout.overflow) problems.push(`${name}: horizontal overflow`);
      if (layout.clippedActions.length) {
        problems.push(
          `${name}: clipped actions without horizontal scrolling: ${
            [...new Set(layout.clippedActions)].join(", ")
          }`,
        );
      }
      await page.screenshot({
        path: `${SCREENSHOT_DIR}/${name}.png`,
        fullPage: true,
      });
    }
    try {
      for (
        const viewport of [
          { width: 390, height: 844, name: "mobile" },
          { width: 1440, height: 1000, name: "desktop" },
        ]
      ) {
        const context = await browser.createBrowserContext();
        try {
          const page = await context.newPage();
          await page.setViewport({
            width: viewport.width,
            height: viewport.height,
          });
          for (
            const [path, name] of [["/", "quests"], ["/bounty", "bounty"], [
              "/highscore",
              "highscore",
            ], ["/editor", "login"]]
          ) {
            await preparePage(page, path);
            await page.waitForSelector("h1", { visible: true, timeout: WAIT });
            await capture(page, `${name}-${viewport.name}`);
          }
          await page.type("#password", "dev");
          await page.click(".auth-submit");
          await page.waitForSelector(".editor-container", {
            visible: true,
            timeout: WAIT,
          });
          for (const label of ["Quests", "Rewards", "Settings"]) {
            await page.$$eval(".editor-tab", (tabs, label) => {
              const tab = tabs.find((e) => e.textContent?.includes(label));
              if (!(tab instanceof HTMLButtonElement)) {
                throw new Error(`Missing ${label} tab`);
              }
              tab.click();
            }, label);
            await page.waitForSelector(`#${label.toLowerCase()}-panel`, {
              visible: true,
              timeout: WAIT,
            });
            await capture(
              page,
              `editor-${label.toLowerCase()}-${viewport.name}`,
            );
          }
        } finally {
          await context.close();
        }
      }
      assertEquals(problems, []);
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "invalid day and unknown route return diagnostic error pages",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      for (
        const testCase of [
          { path: "/day/not-a-date", status: 400, marker: "Wrong Day!" },
          { path: "/route-that-does-not-exist", status: 404, marker: "Gone." },
        ]
      ) {
        const page = await browser.newPage();
        try {
          await setTimezoneCookie(page);
          await page.evaluateOnNewDocument(() =>
            localStorage.setItem("quest-log-first-time", "true")
          );
          const runtime = watchRuntime(page);
          const response = await page.goto(`${BASE_URL}${testCase.path}`, {
            waitUntil: "domcontentloaded",
            timeout: WAIT,
          });
          assertEquals(response?.status(), testCase.status, testCase.path);
          await page.waitForFunction(
            (marker: string) =>
              document.body.textContent?.includes(marker) === true,
            { timeout: WAIT },
            testCase.marker,
          );
          const diagnostics = await runtime.diagnostics();
          assert(
            diagnostics.failures.some((failure) =>
              failure.url.endsWith(testCase.path) &&
              failure.status === testCase.status
            ),
            `${testCase.path}: missing response payload diagnostic ${
              JSON.stringify(diagnostics.failures)
            }`,
          );
          assertEquals(
            diagnostics.errors.filter((error) =>
              !error.includes(`status of ${testCase.status}`)
            ),
            [],
            `${testCase.path}: ${JSON.stringify(diagnostics.errors)}`,
          );
          assertEquals(
            diagnostics.failures.filter((failure) =>
              !failure.url.endsWith(testCase.path)
            ),
            [],
            `${testCase.path}: unrelated failed requests ${
              JSON.stringify(diagnostics.failures)
            }`,
          );
        } finally {
          await page.close();
        }
      }
    } finally {
      await browser.close();
    }
  },
});
