# kage-zed

Kage(Ebitengine shader 语言)的 [Zed editor](https://zed.dev) 扩展,基于
[sedyh/ebitengine-kage-vscode](./ebitengine-kage-vscode) 移植并增强。

## 功能

- **语法高亮**:基于自带的 tree-sitter-kage 语法(支持 uniforms、swizzling、
  矩阵/向量类型、`imageSrcNAt` 系列内建函数等)
- **代码补全**(由内置的 `kage-ls` 语言服务器提供):
  - 全部内建函数(带签名 + 用法文档),来自 ebitengine-kage-vscode 的函数表
  - 内建类型(vec2/vec3/vec4/ivec2-4/mat2-4)、关键字
  - `.` 触发的向量分量 swizzle 补全(`.rgb`、`.xyzw` 等)
  - 当前文件中的标识符(uniform 变量、局部变量等)补全
- **Hover 文档**:内建函数与类型上显示签名、说明和文档链接
- **代码片段**:Fragment/Vertex 着色器模板、imageColor 辅助函数模板等

## 目录结构

```
kage-zed/
├── extension.toml          # 扩展清单
├── src/lib.rs              # 扩展入口(注册语言服务器命令)
├── languages/kage/         # 语言配置 + 高亮查询
├── grammar-src/            # tree-sitter-kage 语法(独立 git 仓库,Zed 构建时 checkout)
├── snippets/Kage.json      # 代码片段(按语言名 "Kage" 匹配)
├── crates/kage-ls/         # Kage 语言服务器(补全/hover,原生 Rust 二进制)
└── ebitengine-kage-vscode/ # 移植来源(VSCode 扩展,仅作参考)
```

## 安装

1. 安装语言服务器(补全必需):

   ```
   cargo install --path crates/kage-ls
   ```

2. 在 Zed 中安装扩展:命令面板(Ctrl-Shift-P)→ `install dev extension`
   → 选择本目录(`kage-zed`)。Zed 会自动:
   - 把 Rust 扩展编译为 wasm
   - 下载 wasi-sdk 并从 `extension.toml` 中 `repository` 指向的 git 仓库
     checkout 并编译 tree-sitter 语法

3. 打开任意 `.kage` 文件即可使用。

### 移动目录后

`extension.toml` 中 `[grammars.kage] repository` 使用了本机的绝对路径
(指向 `grammar-src/`)。如果移动了项目位置,请更新该路径:

```toml
[grammars.kage]
repository = "新的绝对路径/kage-zed/grammar-src"
rev = "main"
```

发布到扩展市场时,把 `grammar-src` 推送到 GitHub 并改用仓库 URL 与 commit。

## 更新语法

修改 `grammar-src/grammar.js` 后:

```
cd grammar-src
tree-sitter generate        # 需要 tree-sitter CLI(npm i -g tree-sitter)
tree-sitter parse test.kage # 验证
git add -A && git commit -m "..."   # Zed 通过 git fetch 拉取更新
```

然后在 Zed 中重新 `install dev extension`。

## 测试语言服务器

```
cd crates/kage-ls
cargo build
node lsp_smoke_test.js
```

