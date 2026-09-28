// End-to-end smoke test for kage-ls over stdio.
const { spawn, execSync } = require("child_process");
const path = require("path");

const binaryName = process.platform === "win32" ? "kage-ls.exe" : "kage-ls";
let serverPath = path.join(__dirname, "target", "debug", binaryName);
try {
  const targetDir = JSON.parse(execSync("cargo metadata --format-version 1 --no-deps", { cwd: __dirname }).toString()).target_directory;
  serverPath = path.join(targetDir, "debug", binaryName);
} catch (_) {}
const child = spawn(serverPath, [], { stdio: ["pipe", "pipe", "inherit"] });

let buffer = Buffer.alloc(0);
const pending = new Map();
let nextId = 1;

child.stdout.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const headerEnd = buffer.indexOf("\r\n\r\n");
    if (headerEnd < 0) break;
    const header = buffer.slice(0, headerEnd).toString();
    const length = parseInt(/Content-Length: (\d+)/.exec(header)[1], 10);
    const total = headerEnd + 4 + length;
    if (buffer.length < total) break;
    const body = buffer.slice(headerEnd + 4, total).toString();
    buffer = buffer.slice(total);
    const message = JSON.parse(body);
    if (message.id && pending.has(message.id)) {
      pending.get(message.id)(message);
      pending.delete(message.id);
    }
  }
});

function send(method, params) {
  return new Promise((resolve) => {
    const id = nextId++;
    const body = JSON.stringify({ jsonrpc: "2.0", id, method, params });
    child.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
    pending.set(id, resolve);
  });
}

function notify(method, params) {
  const body = JSON.stringify({ jsonrpc: "2.0", method, params });
  child.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

const text = `package main

// vertex shader uniforms
var Glow float

func Fragment(pos vec4, tex vec2, col vec4) vec4 {
\tcol2 := imageSrc0At(tex)
\tvalue := mix(col2.rgb, Glow, 0.5)
\treturn vec4(valu, 0.0, 0.0, 1.0)
}
`;

function positionAfter(snippet) {
  const idx = text.indexOf(snippet);
  const after = text.slice(0, idx + snippet.length);
  const lineStart = after.lastIndexOf("\n") + 1;
  return { line: after.slice(0, lineStart).split("\n").length - 1, character: idx + snippet.length - lineStart };
}

function positionBefore(snippet) {
  const pos = positionAfter(snippet);
  return { line: pos.line, character: pos.character - snippet.length };
}

(async () => {
  const init = await send("initialize", { capabilities: {} });
  console.log("init serverInfo:", JSON.stringify(init.result.serverInfo));

  notify("initialized", {});
  notify("textDocument/didOpen", {
    textDocument: { uri: "file:///test.kage", languageId: "kage", version: 1, text },
  });

  // Typing "valu" inside vec4(...)
  const comp = await send("textDocument/completion", {
    textDocument: { uri: "file:///test.kage" },
    position: positionAfter("valu"),
  });
  const items = comp.result.items;
  console.log("completions for 'valu':", items.map((i) => i.label).join(", "));

  // Typing "imag" prefix
  const comp2 = await send("textDocument/completion", {
    textDocument: { uri: "file:///test.kage" },
    position: positionAfter("imageSrc"),
  });
  console.log(
    "completions for 'imageSrc' (top 8):",
    comp2.result.items.map((i) => i.label).slice(0, 8).join(", ")
  );

  // Swizzle after "col2." -> position right after the dot
  const swzPos = (() => {
    const dot = text.indexOf("col2.");
    const before = text.slice(0, dot + 5);
    const line = before.split("\n").length - 1;
    const character = before.length - before.lastIndexOf("\n") - 1;
    return { line, character };
  })();
  const swz = await send("textDocument/completion", {
    textDocument: { uri: "file:///test.kage" },
    position: swzPos,
  });
  console.log(
    "swizzle after '.':",
    swz.result.items.map((i) => i.label).join(", ")
  );

  // Hover over "mix"
  const mixPos = positionAfter("mix");
  const hov = await send("textDocument/hover", {
    textDocument: { uri: "file:///test.kage" },
    position: positionAfter("mix"),
  });
  console.log("hover for 'mix':", hov.result ? hov.result.contents.value.slice(0, 150) : "none");

  // Hover over a type
  const vec4Pos = positionAfter("vec4) vec4");
  const hov2 = await send("textDocument/hover", {
    textDocument: { uri: "file:///test.kage" },
    position: positionAfter("vec4) vec4"),
  });
  console.log("hover for 'vec4':", hov2.result ? hov2.result.contents.value.slice(0, 100) : "none");

  const shutdown = await send("shutdown", null);
  console.log("shutdown ok:", shutdown.result === null);
  notify("exit", null);
  child.stdin.end();
  setTimeout(() => process.exit(0), 200);
})();
