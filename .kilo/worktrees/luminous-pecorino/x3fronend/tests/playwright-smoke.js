async function main() {
  let playwright;
  try {
    playwright = require("playwright");
  } catch {
    process.stderr.write(
      "Playwright is not installed in this workspace. Install it before running smoke tests.\n",
    );
    process.exit(1);
  }

  const { chromium } = playwright;
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  const baseUrl = process.env.X3_SITE_BASE_URL || "http://127.0.0.1:4173";
  const pages = [
    "x3star-landing.html",
    "x3star-dashboard.html",
    "x3star-governance.html",
    "x3star-node-health.html",
    "x3star-staking.html",
  ];

  try {
    for (const file of pages) {
      await page.goto(`${baseUrl}/${file}`, { waitUntil: "networkidle" });
      const title = await page.title();
      if (!title) {
        throw new Error(`No title rendered for ${file}`);
      }
    }
  } finally {
    await browser.close();
  }
}

main().catch((error) => {
  process.stderr.write(`${error.stack || error.message}\n`);
  process.exit(1);
});
