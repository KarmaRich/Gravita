/**
 * 纯原生 Node.js CRX3 (Chrome/Edge 浏览器扩展包 .crx) 签名打包工具（零第三方 npm 依赖）
 * 用法: node scripts/pack-crx.mjs <输入.zip路径> <输出.crx路径> [可选.pem私钥路径]
 */

import crypto from "node:crypto";
import fs from "node:fs";

function encodeVarint(num) {
  const bytes = [];
  let n = num >>> 0;
  while (n >= 0x80) {
    bytes.push((n & 0x7f) | 0x80);
    n >>>= 7;
  }
  bytes.push(n);
  return Buffer.from(bytes);
}

function encodeProtobufBytesField(fieldNumber, payload) {
  const tag = encodeVarint((fieldNumber << 3) | 2);
  const len = encodeVarint(payload.length);
  return Buffer.concat([tag, len, payload]);
}

export function buildCrx3(zipBuffer, privateKeyPem = null) {
  let privateKey;
  if (privateKeyPem && fs.existsSync(privateKeyPem)) {
    privateKey = crypto.createPrivateKey(fs.readFileSync(privateKeyPem, "utf8"));
  } else {
    const pair = crypto.generateKeyPairSync("rsa", { modulusLength: 2048 });
    privateKey = pair.privateKey;
  }

  const publicKeyDer = crypto
    .createPublicKey(privateKey)
    .export({ type: "spki", format: "der" });

  // 1. 计算 16 字节 crx_id = SHA256(publicKeyDer)[0..16]
  const crxId = crypto.createHash("sha256").update(publicKeyDer).digest().subarray(0, 16);

  // 2. 构造 SignedData protobuf (field 1: crx_id)
  const signedData = encodeProtobufBytesField(1, crxId);

  // 3. 计算 RSA-SHA256 签名: "CRX3 SignedData\0" + uint32_le(signedData.length) + signedData + zipBuffer
  const prefix = Buffer.from("CRX3 SignedData\x00", "utf8");
  const signedLenLe = Buffer.alloc(4);
  signedLenLe.writeUInt32LE(signedData.length, 0);

  const signer = crypto.createSign("RSA-SHA256");
  signer.update(prefix);
  signer.update(signedLenLe);
  signer.update(signedData);
  signer.update(zipBuffer);
  const signature = signer.sign(privateKey);

  // 4. 构造 AsymmetricKeyProof protobuf (field 1: public_key, field 2: signature)
  const keyProof = Buffer.concat([
    encodeProtobufBytesField(1, publicKeyDer),
    encodeProtobufBytesField(2, signature),
  ]);

  // 5. 构造 CrxFileHeader protobuf (field 2: sha256_with_rsa, field 10000: signed_header_data)
  const crxHeader = Buffer.concat([
    encodeProtobufBytesField(2, keyProof),
    encodeProtobufBytesField(10000, signedData),
  ]);

  // 6. 组装最终 CRX3 二进制: "Cr24" + version(3) + header_len + crxHeader + zipBuffer
  const magic = Buffer.from("Cr24", "utf8");
  const versionAndHeaderLen = Buffer.alloc(8);
  versionAndHeaderLen.writeUInt32LE(3, 0);
  versionAndHeaderLen.writeUInt32LE(crxHeader.length, 4);

  return Buffer.concat([magic, versionAndHeaderLen, crxHeader, zipBuffer]);
}

if (process.argv[2] && process.argv[3]) {
  const zipPath = process.argv[2];
  const crxPath = process.argv[3];
  const pemPath = process.argv[4] || null;
  const zipBuf = fs.readFileSync(zipPath);
  const crxBuf = buildCrx3(zipBuf, pemPath);
  fs.writeFileSync(crxPath, crxBuf);
  console.log(`✔ 已生成 CRX3 浏览器扩展包: ${crxPath} (${crxBuf.length} bytes)`);
}
