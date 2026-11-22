import test from "node:test";
import assert from "node:assert/strict";
import { shouldInterceptDownload, DEFAULT_INTERCEPT_RULE } from "./filter.ts";

test("匹配常见后缀且超过大小阈值的 HTTP 下载应被拦截", () => {
  const result = shouldInterceptDownload({
    url: "https://cdn.example.com/release/app.zip",
    filename: "app.zip",
    fileSize: 5 * 1024 * 1024,
  });
  assert.equal(result, true);
});

test("小于最小拦截阈值的文件不应被拦截", () => {
  const result = shouldInterceptDownload({
    url: "https://cdn.example.com/tiny.zip",
    filename: "tiny.zip",
    fileSize: 512 * 1024,
  });
  assert.equal(result, false);
});

test("命中排除域名的下载不应被拦截", () => {
  const result = shouldInterceptDownload(
    {
      url: "https://files.internal.local/test.iso",
      filename: "test.iso",
      fileSize: 100 * 1024 * 1024,
    },
    {
      ...DEFAULT_INTERCEPT_RULE,
      excludedDomains: ["internal.local"],
    }
  );
  assert.equal(result, false);
});

test("blob 协议链接不应被拦截", () => {
  const result = shouldInterceptDownload({
    url: "blob:https://example.com/1234-5678",
    filename: "export.zip",
    fileSize: 10 * 1024 * 1024,
  });
  assert.equal(result, false);
});
