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
  const executablePath = "/usr/bin/firefox";
  const launchArgs = [];
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
  });
}

async function chrome(isCI: boolean, headless: boolean) {
  const executablePath = "/usr/bin/google-chrome"; // Or google-chrome-stable
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

export async function newPage(browser: puppeteer.Browser) {
  const page = await browser.newPage();
  await page.goto("http://localhost:3000");
  return page;
}
