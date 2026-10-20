// どうぶつしょうぎの通し対局を Chrome で駆動する。人間側はランダムに指す。
// 使い方: NODE_PATH=<playwright-core のある node_modules> node drive.cjs [--side sente|gote] [--level 1|2|3] [--out dir] [--url url]
const { chromium } = require("playwright-core");
const fs = require("fs");

const arg = (k, d) => {
  const i = process.argv.indexOf(`--${k}`);
  return i >= 0 ? process.argv[i + 1] : d;
};
const side = arg("side", "sente");
const level = Number(arg("level", "3"));
const out = arg("out", "shots");
const url = arg("url", "http://localhost:5173/");
fs.mkdirSync(out, { recursive: true });

(async () => {
  const browser = await chromium.launch({ channel: "chrome" });
  const page = await (await browser.newContext({ viewport: { width: 390, height: 800 } })).newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push("pageerror: " + e.message));
  page.on("response", (r) => r.status() >= 400 && errors.push(`${r.status()} ${r.url()}`));

  await page.goto(url);
  await page.waitForSelector(".board .cell");
  await page.selectOption("select >> nth=0", { index: side === "gote" ? 1 : 0 });
  await page.selectOption("select >> nth=1", { index: level - 1 });
  await page.click("text=新しい対局");
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${out}/1-start.png` });

  const cells = page.locator(".board .cell");
  const header = async () => (await page.locator("body").innerText()).split("後手(CPU)")[0];
  let turns = 0;
  let finished = false;
  for (; turns < 120; turns++) {
    const head = await header();
    if (/勝ち|引き分け/.test(head)) { finished = true; break; }
    if (!head.includes("あなたの番")) { await page.waitForTimeout(500); continue; }
    for (let i = 0; i < 12; i++) {
      await cells.nth(i).click();
      const t = page.locator(".cell.target");
      if (await t.count()) { await t.nth(Math.floor(Math.random() * (await t.count()))).click(); break; }
    }
    await page.waitForTimeout(600);
  }
  await page.screenshot({ path: `${out}/2-end.png` });
  console.log(`side=${side} level=${level} turns=${turns} finished=${finished}`);
  console.log("result:", (await header()).split("\n").slice(0, 3).join(" / "));
  console.log("errors:", JSON.stringify(errors));
  await browser.close();
  process.exit(finished && errors.length === 0 ? 0 : 1);
})();
