import puppeteer from "puppeteer-core";

export async function launchBrowser() {
  const isCI = Deno.env.get("CI") === "true";
  const headless = Deno.env.get("HEADLESS") === "true";
  const browser = Deno.env.get("BROWSER") || "chrome";
  if (browser === "chrome") {
    return await chrome(isCI, headless);
  } else if (browser === "firefox") {
    return await firefox(isCI, headless);
  } else {
    throw new Error(`Unsupported browser: ${browser}`);
  }
}

async function firefox(isCI: boolean, headless: boolean) {
  const executablePath = Deno.env.get("BROWSER_EXECUTABLE") ?? "/usr/bin/firefox";
  const userDataDir = await Deno.makeTempDir({ prefix: "quest-log-firefox-" });
  const launchArgs = ["--profile", userDataDir];
  if (isCI) {
    // In CI (GitHub Actions, GitLab, etc.), we usually run in containers
    // as root or without user namespaces, so sandbox is required.
    launchArgs.push("--no-sandbox");
    console.log("Launching firefox in CI mode (no-sandbox enabled)");
  } else {
    // Locally, we try to use the sandbox for security.
    console.log("Launching firefox in Local mode (sandbox enabled)");
  }
  return await puppeteer.launch({
    executablePath: executablePath,
    headless: headless,
    args: launchArgs,
    userDataDir,
  });
}

async function chrome(isCI: boolean, headless: boolean) {
  const executablePath = Deno.env.get("BROWSER_EXECUTABLE") ?? chromeExecutable();
  const launchArgs = [
    "--disable-dev-shm-usage", // General Linux stability
  ];
  if (isCI) {
    // In CI (GitHub Actions, GitLab, etc.), we usually run in containers
    // as root or without user namespaces, so sandbox is required.
    launchArgs.push("--no-sandbox", "--disable-setuid-sandbox");
    console.log("Launching chrome in CI mode (no-sandbox enabled)");
  } else {
    // Locally, we try to use the sandbox for security.
    console.log("Launching chrome in Local mode (sandbox enabled)");
  }
  return await puppeteer.launch({
    executablePath: executablePath,
    headless: headless,
    args: launchArgs,
  });
}

function chromeExecutable() {
  for (const path of [
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
  ]) {
    try {
      Deno.statSync(path);
      return path;
    } catch {
      // try next common Linux browser path
    }
  }
  return "/usr/bin/google-chrome";
}

export async function newPage(browser: puppeteer.Browser) {
  const page = await browser.newPage();
  await page.goto("http://localhost:3000");
  return page;
}

export async function clearBrowserState(page: puppeteer.Page) {
  await page.evaluate(() => {
    localStorage.clear();
    sessionStorage.clear();
  });
  await page.reload();
}

export async function typeIntoInput(
  page: puppeteer.Page,
  selector: string,
  text: string,
) {
  await page.focus(selector);
  await page.keyboard.type(text);
}
