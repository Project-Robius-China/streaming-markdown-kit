---
title: Markdown 全样式示例文档
author: ChatGPT
date: 2026-04-18
tags:
  - Markdown
  - Mermaid
  - Rust
  - 文档示例
---

# Markdown 全样式示例文档

> 这是一份用于测试 Markdown 渲染能力的示例文档，包含标题、文本样式、列表、表格、代码块、Mermaid 图表、数学公式、脚注、折叠内容等。

---

## 1. 标题

# 一级标题

## 二级标题

### 三级标题

#### 四级标题

##### 五级标题

###### 六级标题

---

## 2. 文本样式

普通文本。

**加粗文本**

*斜体文本*

***加粗且斜体***

~~删除线~~

`行内代码`

==高亮文本==

<u>下划线文本，依赖 HTML 支持</u>

H~2~O

x^2^

中文、English、にほんご、한국어、Русский、Español。

---

## 3. 引用

> 这是一级引用。
>
> > 这是嵌套引用。
> >
> > > 这是三级引用。

---

## 4. GitHub 风格提示块

> [!NOTE]
> 这是一个 NOTE 提示块。

> [!TIP]
> 这是一个 TIP 提示块。

> [!IMPORTANT]
> 这是一个 IMPORTANT 提示块。

> [!WARNING]
> 这是一个 WARNING 提示块。

> [!CAUTION]
> 这是一个 CAUTION 提示块。

---

## 5. 分割线

---

***

___

---

## 6. 无序列表

- 苹果
- 香蕉
- 橙子
  - 血橙
  - 脐橙
    - 赣南脐橙
    - 新奇士橙

---

## 7. 有序列表

1. 安装 Rust
2. 创建项目
3. 编写代码
4. 运行测试
5. 发布版本

---

## 8. 混合列表

1. 前端
   - HTML
   - CSS
   - JavaScript
2. 后端
   - Rust
   - Go
   - Java
3. 数据库
   - PostgreSQL
   - Redis
   - SQLite

---

## 9. 任务列表

- [x] 编写需求文档
- [x] 设计数据结构
- [ ] 实现核心逻辑
- [ ] 编写单元测试
- [ ] 发布 v1.0.0

---

## 10. 链接

这是一个普通链接：

[OpenAI](https://openai.com)

这是一个带标题的链接：

[Markdown Guide](https://www.markdownguide.org "Markdown Guide")

自动链接：

<https://example.com>

邮箱链接：

<hello@example.com>

---

## 11. 图片

普通图片：

![Markdown Logo](https://markdown-here.com/img/icon256.png)

带链接的图片：

[![Rust Logo](https://www.rust-lang.org/logos/rust-logo-512x512.png)](https://www.rust-lang.org)

---

## 12. 表格

| 编号 | 语言       | 类型             | 适合场景             | 推荐指数 |
| ---: | :--------- | :--------------- | :------------------- | :------: |
| 1    | Rust       | 系统编程语言     | 高性能、安全、并发   | ⭐⭐⭐⭐⭐ |
| 2    | Go         | 后端服务语言     | 云原生、微服务       | ⭐⭐⭐⭐ |
| 3    | Python     | 脚本 / AI 语言   | 数据科学、自动化     | ⭐⭐⭐⭐ |
| 4    | TypeScript | 前端 / 全栈语言  | Web 应用、大型前端   | ⭐⭐⭐⭐ |
| 5    | C++        | 系统 / 游戏开发  | 引擎、底层、高性能   | ⭐⭐⭐⭐ |

---

## 13. 复杂表格

| 模块 | 功能 | 输入 | 输出 | 状态 |
| :--- | :--- | :--- | :--- | :--- |
| Parser | 解析 Markdown | `.md` 文件 | AST | ✅ 完成 |
| Renderer | 渲染 HTML | AST | HTML 字符串 | 🚧 开发中 |
| Exporter | 导出 PDF | HTML | PDF 文件 | ⏳ 计划中 |
| Mermaid | 渲染图表 | Mermaid 代码块 | SVG / Canvas | ✅ 完成 |

---

## 14. 行内代码

可以使用 `cargo new demo` 创建 Rust 项目。

可以使用 `println!("Hello, Markdown!");` 输出内容。

---

## 15. 普通代码块

```text
这是一段普通文本代码块。
Markdown 渲染器会保留缩进、换行和空格。
```

---

## 16. Rust 代码块

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct User {
    id: u64,
    name: String,
    email: String,
}

fn main() {
    let mut users: HashMap<u64, User> = HashMap::new();

    users.insert(
        1,
        User {
            id: 1,
            name: String::from("Alice"),
            email: String::from("alice@example.com"),
        },
    );

    for user in users.values() {
        println!("{:?}", user);
    }
}
```

---

## 17. JavaScript 代码块

```javascript
const users = [
  { id: 1, name: "Alice", role: "admin" },
  { id: 2, name: "Bob", role: "user" },
];

const admins = users.filter(user => user.role === "admin");

console.log(admins);
```

---

## 18. Python 代码块

```python
from dataclasses import dataclass

@dataclass
class User:
    id: int
    name: str
    email: str

users = [
    User(1, "Alice", "alice@example.com"),
    User(2, "Bob", "bob@example.com"),
]

for user in users:
    print(user)
```

---

## 19. Bash 代码块

```bash
#!/usr/bin/env bash

set -euo pipefail

cargo build --release
cargo test
cargo clippy -- -D warnings
```

---

## 20. JSON 代码块

```json
{
  "name": "markdown-demo",
  "version": "1.0.0",
  "features": ["table", "code", "mermaid", "math"],
  "enabled": true
}
```

---

## 21. YAML 代码块

```yaml
server:
  host: 127.0.0.1
  port: 8080

database:
  url: postgres://user:password@localhost:5432/app
  pool_size: 10

features:
  mermaid: true
  markdown: true
  export_pdf: false
```

---

## 22. TOML 代码块

```toml
[package]
name = "markdown-demo"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

---

## 23. Diff 代码块

```diff
- let name = "old";
+ let name = "new";

- println!("Hello");
+ println!("Hello, Markdown!");
```

---

## 24. HTML 混排

<div align="center">

### 居中文本

<strong>这是一段 HTML + Markdown 混排内容。</strong>

</div>

<details>
<summary>点击展开详情</summary>

这里是折叠内容。

- 支持列表
- 支持代码
- 支持表格

```rust
fn hidden_code() {
    println!("This is hidden in details.");
}
```

</details>

---

## 25. 数学公式

行内公式：$E = mc^2$

块级公式：

$$
\int_{-\infty}^{+\infty} e^{-x^2} dx = \sqrt{\pi}
$$

矩阵：

$$
A =
\begin{bmatrix}
1 & 2 & 3 \\
4 & 5 & 6 \\
7 & 8 & 9
\end{bmatrix}
$$

---

## 26. Mermaid 流程图

```mermaid
flowchart TD
    A[开始] --> B{是否已安装 Rust?}
    B -- 是 --> C[创建项目]
    B -- 否 --> D[安装 Rust]
    D --> C
    C --> E[编写代码]
    E --> F[运行 cargo test]
    F --> G{测试是否通过?}
    G -- 是 --> H[发布版本]
    G -- 否 --> E
    H --> I[结束]
```

---

## 27. Mermaid 时序图

```mermaid
sequenceDiagram
    participant User as 用户
    participant UI as 前端
    participant API as 后端 API
    participant DB as 数据库

    User->>UI: 点击登录
    UI->>API: POST /login
    API->>DB: 查询用户信息
    DB-->>API: 返回用户记录
    API-->>UI: 返回 JWT Token
    UI-->>User: 登录成功
```

---

## 28. Mermaid 类图

```mermaid
classDiagram
    class User {
        +u64 id
        +String name
        +String email
        +login()
        +logout()
    }

    class Admin {
        +deleteUser()
        +createUser()
    }

    class Guest {
        +browse()
    }

    User <|-- Admin
    User <|-- Guest
```

---

## 29. Mermaid 状态图

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Loading: fetch()
    Loading --> Success: ok
    Loading --> Error: failed
    Success --> Idle: reset()
    Error --> Loading: retry()
    Error --> Idle: cancel()
```

---

## 30. Mermaid ER 图

```mermaid
erDiagram
    USER ||--o{ ORDER : places
    ORDER ||--|{ ORDER_ITEM : contains
    PRODUCT ||--o{ ORDER_ITEM : included_in

    USER {
        int id
        string name
        string email
    }

    ORDER {
        int id
        int user_id
        datetime created_at
    }

    PRODUCT {
        int id
        string name
        decimal price
    }

    ORDER_ITEM {
        int id
        int order_id
        int product_id
        int quantity
    }
```

---

## 31. Mermaid 甘特图

```mermaid
gantt
    title 项目开发计划
    dateFormat  YYYY-MM-DD
    section 设计
    需求分析        :done,    des1, 2026-04-01, 2026-04-03
    架构设计        :active,  des2, 2026-04-04, 2026-04-07

    section 开发
    后端开发        :dev1, 2026-04-08, 2026-04-15
    前端开发        :dev2, 2026-04-10, 2026-04-18

    section 测试
    集成测试        :test1, 2026-04-19, 2026-04-22
    发布准备        :test2, 2026-04-23, 2026-04-25
```

---

## 32. Mermaid 饼图

```mermaid
pie title 技术栈占比
    "Rust" : 35
    "TypeScript" : 30
    "Python" : 20
    "SQL" : 10
    "Other" : 5
```

---

## 33. Mermaid Git Graph

```mermaid
gitGraph
    commit id: "init"
    branch dev
    checkout dev
    commit id: "feature-api"
    commit id: "feature-ui"
    checkout main
    merge dev
    commit id: "release-v1"
```

---

## 34. Mermaid Mindmap

```mermaid
mindmap
  root((Markdown))
    文本
      加粗
      斜体
      删除线
    结构
      标题
      列表
      表格
    代码
      行内代码
      代码块
    图表
      Mermaid
      流程图
      时序图
      ER图
```

---

## 35. 脚注

这里有一个脚注引用。[^note]

这里还有第二个脚注。[^rust]

[^note]: 这是脚注内容。
[^rust]: Rust 是一门强调安全性、并发性和性能的系统编程语言。

---

## 36. 定义列表

Markdown  
: 一种轻量级标记语言。

Mermaid  
: 一种使用文本描述图表的工具。

Rust  
: 一种注重内存安全和并发安全的系统编程语言。

---

## 37. Emoji

- ✅ 完成
- 🚧 进行中
- ❌ 失败
- ⚠️ 警告
- 💡 想法
- 🔥 热点
- 🦀 Rust
- 📦 包
- 🚀 发布

---

## 38. 键盘按键

请按 <kbd>Ctrl</kbd> + <kbd>C</kbd> 复制。

请按 <kbd>Ctrl</kbd> + <kbd>V</kbd> 粘贴。

请按 <kbd>Cmd</kbd> + <kbd>Shift</kbd> + <kbd>P</kbd> 打开命令面板。

---

## 39. 上标、下标和缩写

H<sub>2</sub>O

E = mc<sup>2</sup>

<abbr title="HyperText Markup Language">HTML</abbr>

<abbr title="Cascading Style Sheets">CSS</abbr>

---

## 40. 锚点跳转

跳转到：

- [表格](#12-表格)
- [Rust 代码块](#16-rust-代码块)
- [Mermaid 流程图](#26-mermaid-流程图)
- [脚注](#35-脚注)

---

## 41. 转义字符

如果想显示 Markdown 语法本身，可以使用反斜杠转义：

\*这不会变成斜体\*

\*\*这不会变成加粗\*\*

\`这不会变成行内代码\`

\# 这不会变成标题

---

## 42. 嵌套内容综合示例

> ### 引用中的标题
>
> 引用中可以包含列表：
>
> - 第一项
> - 第二项
>
> 也可以包含代码：
>
> ```rust
> fn quoted_code() {
>     println!("code in quote");
> }
> ```

---

## 43. API 文档示例

### `GET /api/users`

获取用户列表。

#### 请求参数

| 参数 | 类型 | 必填 | 描述 |
| :--- | :--- | :---: | :--- |
| page | number | 否 | 页码 |
| size | number | 否 | 每页数量 |
| keyword | string | 否 | 搜索关键词 |

#### 响应示例

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "items": [
      {
        "id": 1,
        "name": "Alice",
        "email": "alice@example.com"
      }
    ],
    "total": 1
  }
}
```

---

## 44. Rust 项目 README 示例片段

### 安装

```bash
cargo install markdown-demo
```

### 使用

```bash
markdown-demo input.md --output output.html
```

### 配置

```toml
[render]
theme = "github"
enable_mermaid = true
enable_math = true
```

### 运行测试

```bash
cargo test
```

---

## 45. Check List 示例

### 发布前检查

- [x] 单元测试通过
- [x] 集成测试通过
- [x] 文档已更新
- [ ] CHANGELOG 已更新
- [ ] Git tag 已创建
- [ ] Release note 已发布

---

## 46. 目录示例

```text
project/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   └── parser.rs
├── tests/
│   └── integration_test.rs
└── docs/
    └── index.md
```

---

## 47. Badge 示例

![Version](https://img.shields.io/badge/version-1.0.0-blue)

![Build](https://img.shields.io/badge/build-passing-brightgreen)

![License](https://img.shields.io/badge/license-MIT-green)

---

## 48. 内嵌 SVG

<svg width="240" height="80" xmlns="http://www.w3.org/2000/svg">
  <rect width="240" height="80" rx="12" fill="#f5f5f5"/>
  <text x="120" y="48" text-anchor="middle" font-size="22">
    Markdown SVG
  </text>
</svg>

---

## 49. 注释

下面是一段 HTML 注释，渲染时通常不可见：

<!-- 这是注释内容 -->

---

## 50. 结尾

感谢阅读这份 Markdown 全样式示例文档。

> Markdown 的具体渲染效果取决于平台，例如 GitHub、GitLab、Obsidian、Typora、VS Code、Notion、Docsify、MkDocs、Docusaurus 等对扩展语法的支持并不完全一致。
