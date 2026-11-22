import test from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { DEFAULT_CONFIG, buildDownloadHeaders, shouldInterceptDownload } from "./filter.js";
import { checkDesktopHealth, dispatchToDesktop } from "./bridge.js";

test("匹配常见后缀且超过大小阈值的 HTTP 下载应被拦截", () => {
  assert.equal(
    shouldInterceptDownload({
      url: "https://cdn.example.com/release/app.zip",
      filename: "app.zip",
      fileSize: 5 * 1024 * 1024,
    }),
    true
  );
});

test("小于最小拦截阈值的文件不应被拦截", () => {
  assert.equal(
    shouldInterceptDownload({
      url: "https://cdn.example.com/tiny.zip",
      filename: "tiny.zip",
      fileSize: 512 * 1024,
    }),
    false
  );
});

test("命中排除域名或非 HTTP 协议不应被拦截", () => {
  assert.equal(
    shouldInterceptDownload(
      { url: "https://files.internal.local/test.iso", filename: "test.iso", fileSize: 10_000_000 },
      { ...DEFAULT_CONFIG, excludedDomains: ["internal.local"] }
    ),
    false
  );
  assert.equal(
    shouldInterceptDownload({ url: "blob:https://example.com/abc", filename: "a.zip", fileSize: 10_000_000 }),
    false
  );
});

test("请求头组装应正确包含 Referer / User-Agent / Cookie", () => {
  const headers = buildDownloadHeaders({
    referer: "https://example.com/",
    userAgent: "TestUA/1.0",
    cookie: "sid=123",
  });
  assert.deepEqual(headers, [
    "Referer: https://example.com/",
    "User-Agent: TestUA/1.0",
    "Cookie: sid=123",
  ]);
});

test("bridge 模块应正确检测桌面端健康状态并转发下载请求", async () => {
  let receivedBody = null;
  let receivedToken = null;

  const server = http.createServer((req, res) => {
    if (req.method === "GET" && req.url === "/api/health") {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ status: "ok" }));
      return;
    }
    if (req.method === "POST" && req.url === "/api/downloads") {
      receivedToken = req.headers["x-idm-token"];
      let raw = "";
      req.on("data", (chunk) => (raw += chunk));
      req.on("end", () => {
        receivedBody = JSON.parse(raw);
        res.writeHead(202, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ accepted: true }));
      });
      return;
    }
    res.writeHead(404).end();
  });

  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address();
  const endpoint = `http://127.0.0.1:${port}`;

  try {
    assert.equal(await checkDesktopHealth(endpoint), true);
    const ok = await dispatchToDesktop(endpoint, "tok-99", {
      url: "https://example.com/demo.zip",
      filename: "demo.zip",
      headers: ["Cookie: sid=1"],
      auto_start: true,
    });
    assert.equal(ok, true);
    assert.equal(receivedToken, "tok-99");
    assert.equal(receivedBody.url, "https://example.com/demo.zip");
  } finally {
    server.close();
  }
});
