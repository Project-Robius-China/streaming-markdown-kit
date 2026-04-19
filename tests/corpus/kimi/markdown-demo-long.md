```markdown
# 综合 Markdown 元素演示文档

本文档展示 **GitHub Flavored Markdown** 的全部核心能力，涵盖数学公式、多类型图表、代码高亮及表格布局。

---

## 1. 数学公式

### 行内公式
质能方程 E=mc^2 与欧拉公式 e^{i\pi} + 1 = 0 是物理学中最美的等式。
对于任意向量 \mathbf{v} \in \mathbb{R}^n，其范数定义为 \|\mathbf{v}\| = \sqrt{\sum_{i=1}^{n} v_i^2}。

### 块级公式（微分几何）
高斯-博内定理描述曲面曲率与拓扑的关系：

\int_M K \, dA + \int_{\partial M} k_g \, ds = 2\pi \chi(M)


### 矩阵与线性代数
矩阵乘法示例，其中 \mathbf{A} \in \mathbb{R}^{m \times n}，\mathbf{x} \in \mathbb{R}^n：

\mathbf{A}\mathbf{x} = 
\begin{pmatrix}
a_{11} & a_{12} & \cdots & a_{1n} \\
a_{21} & a_{22} & \cdots & a_{2n} \\
\vdots & \vdots & \ddots & \vdots \\
a_{m1} & a_{m2} & \cdots & a_{mn}
\end{pmatrix}
\begin{pmatrix}
x_1 \\
x_2 \\
\vdots \\
x_n
\end{pmatrix}
= 
\begin{pmatrix}
\sum_{j=1}^{n} a_{1j}x_j \\
\sum_{j=1}^{n} a_{2j}x_j \\
\vdots \\
\sum_{j=1}^{n} a_{mj}x_j
\end{pmatrix}


### 概率论
条件概率与贝叶斯定理：

P(A|B) = \frac{P(B|A)P(A)}{P(B)} \quad \text{其中} \quad P(B) = \sum_{i} P(B|A_i)P(A_i)


---

## 2. Mermaid 图表

### 2.1 基础流程图（算法决策）

```mermaid
flowchart TD
    A([开始]) --> B{输入验证}
    B -->|无效| C[抛出异常<br/>Error 400]:::error
    B -->|有效| D[处理数据]
    D --> E{缓存存在?}
    E -->|是| F[读取缓存]:::cache
    E -->|否| G[查询数据库]:::db
    G --> H[写入缓存]:::cache
    F --> I[返回结果]
    H --> I
    I --> J([结束])
    
    classDef error fill:#881337,stroke:#fb7185,color:#e2e8f0
    classDef cache fill:#7c2d12,stroke:#fb923c,color:#e2e8f0
    classDef db fill:#4c1d95,stroke:#a78bfa,color:#e2e8f0
```

### 2.2 系统架构图（微服务）

```mermaid
flowchart TB
    classDef frontend fill:#083344,stroke:#22d3ee,stroke-width:2px,color:#e2e8f0
    classDef backend fill:#064e3b,stroke:#34d399,stroke-width:2px,color:#e2e8f0
    classDef db fill:#4c1d95,stroke:#a78bfa,stroke-width:2px,color:#e2e8f0
    classDef queue fill:#7c2d12,stroke:#fb923c,stroke-width:2px,color:#e2e8f0
    classDef external fill:#1e293b,stroke:#94a3b8,stroke-width:2px,color:#e2e8f0

    subgraph Client["客户端层"]
        W["Web App<br/>React · TS"]:::frontend
        M["Mobile App<br/>Flutter"]:::frontend
    end
    
    subgraph Gateway["网关层"]
        N["Nginx<br/>反向代理"]:::backend
        K["Kong<br/>JWT · 限流"]:::backend
    end
    
    subgraph Service["业务服务层"]
        U["User Service<br/>Go · gRPC"]:::backend
        O["Order Service<br/>Java · Spring"]:::backend
        P["Payment Service<br/>Node.js"]:::backend
    end
    
    subgraph Message["消息层"]
        R[("Redis<br/>Cache")]:::db
        Q[["RabbitMQ<br/>Async"]]:::queue
    end
    
    subgraph Storage["持久化层"]
        PG[("PostgreSQL<br/>主库")]:::db
        MG[("MongoDB<br/>日志")]:::db
        ES[("Elasticsearch<br/>搜索")]:::db
    end
    
    subgraph Third["第三方服务"]
        S["Stripe API"]:::external
        A["AWS S3"]:::external
    end
    
    W -->|HTTPS| N
    M -->|HTTPS| N
    N --> K
    K -->|gRPC| U
    K -->|REST| O
    K -->|REST| P
    U --> R
    U --> PG
    O --> Q
    Q --> P
    P --> S
    P --> A
    O --> MG
    U --> ES
```

### 2.3 时序图（Sequence Diagram）

```mermaid
sequenceDiagram
    autonumber
    actor U as 用户
    participant C as 客户端
    participant A as API网关
    participant S as 订单服务
    participant P as 支付服务
    participant D as 数据库
    
    U->>C: 提交订单
    C->>A: POST /orders<br/>JWT Token
    A->>A: 验证Token
    
    alt Token无效
        A-->>C: 401 Unauthorized
        C-->>U: 提示登录
    else Token有效
        A->>S: 转发请求
        S->>D: 开始事务
        S->>S: 创建订单记录
        
        par 并行处理
            S->>P: 调用支付接口
            P-->>S: 返回支付URL
        and
            S->>D: 写入订单日志
        end
        
        S->>D: 提交事务
        S-->>A: 订单创建成功
        A-->>C: 201 Created
        C-->>U: 跳转支付页面
    end
```

### 2.4 类图（Class Diagram）

```mermaid
classDiagram
    direction TB
    
    class User {
        -String id
        -String email
        -String password_hash
        +login() Boolean
        +logout() void
        +updateProfile() Boolean
    }
    
    class Order {
        -String orderId
        -Float totalAmount
        -String status
        -DateTime createdAt
        +calculateTotal() Float
        +cancel() Boolean
    }
    
    class Product {
        -String sku
        -String name
        -Float price
        -Integer stock
        +checkAvailability() Boolean
        +reduceStock() void
    }
    
    class Payment {
        -String transactionId
        -String method
        -Float amount
        +process() Boolean
        +refund() Boolean
    }
    
    User "1" --> "*" Order : 拥有
    Order "*" --> "*" Product : 包含
    Order "1" --> "1" Payment : 关联
```

### 2.5 甘特图（Gantt Chart）

```mermaid
gantt
    title 项目开发计划 2024
    dateFormat  YYYY-MM-DD
    section 设计阶段
    需求分析           :done, a1, 2024-01-01, 7d
    系统架构设计       :active, a2, after a1, 5d
    UI/UX设计         :a3, after a2, 10d
    
    section 开发阶段
    后端API开发       :b1, after a3, 15d
    前端页面开发      :b2, after a3, 12d
    数据库设计        :b3, after a2, 3d
    集成测试          :b4, after b1, 5d
    
    section 部署上线
    生产环境配置      :c1, after b4, 3d
    灰度发布          :c2, after c1, 2d
    正式上线          :milestone, after c2, 0d
```

### 2.6 状态图（State Diagram）

```mermaid
stateDiagram-v2
    [*] --> 待支付: 创建订单
    待支付 --> 已取消: 超时/用户取消
    待支付 --> 已支付: 支付成功
    
    已支付 --> 处理中: 确认库存
    处理中 --> 已发货: 物流揽收
    已发货 --> 运输中: 离开仓库
    
    运输中 --> 派送中: 到达城市
    派送中 --> 已签收: 用户签收
    派送中 --> 异常: 联系不上
    
    异常 --> 派送中: 重新预约
    已签收 --> [*]: 完成
    
    已取消 --> [*]: 结束
    已支付 --> 已退款: 申请退款
    已退款 --> [*]: 结束
```

### 2.7 ER 图（实体关系）

```mermaid
erDiagram
    CUSTOMER ||--o{ ORDER : places
    CUSTOMER {
        string id PK
        string email UK
        string name
        string phone
        datetime created_at
    }
    
    ORDER ||--|{ ORDER_ITEM : contains
    ORDER {
        string id PK
        string customer_id FK
        float total_amount
        string status
        datetime order_date
    }
    
    ORDER_ITEM {
        string id PK
        string order_id FK
        string product_id FK
        int quantity
        float unit_price
    }
    
    PRODUCT ||--o{ ORDER_ITEM : "ordered in"
    PRODUCT {
        string id PK
        string sku UK
        string name
        float price
        int stock_level
    }
```

---

## 3. 代码块示例

### Python（数据科学）

```python
import numpy as np
import pandas as pd
from typing import List, Tuple

def gradient_descent(
    X: np.ndarray, 
    y: np.ndarray, 
    lr: float = 0.01, 
    epochs: int = 1000
) -> Tuple[np.ndarray, List[float]]:
    """
    实现线性回归的梯度下降算法
    参数:
        X: 特征矩阵 (m, n)
        y: 目标向量 (m,)
        lr: 学习率 \alpha
        epochs: 迭代次数 T
    返回:
        weights: 权重向量 \theta
        losses: 损失历史 J(\theta)
    """
    m, n = X.shape
    theta = np.zeros(n)
    losses = []
    
    for t in range(epochs):
        # 预测值 \hat{y} = X\theta
        predictions = X @ theta
        errors = predictions - y
        
        # 计算梯度 \nabla J = \frac{1}{m} X^T (X\theta - y)
        gradient = (X.T @ errors) / m
        
        # 更新参数 \theta := \theta - \alpha \nabla J
        theta -= lr * gradient
        
        # 计算 MSE 损失 J(\theta) = \frac{1}{2m} \sum (h_\theta(x^{(i)}) - y^{(i)})^2
        loss = np.mean(errors ** 2) / 2
        losses.append(loss)
        
        if t % 100 == 0:
            print(f"Epoch {t}: Loss = {loss:.4f}")
    
    return theta, losses
```

### Rust（系统编程）

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

/// 线程安全的缓存结构
pub struct Cache<K, V> {
    store: Arc<RwLock<std::collections::HashMap<K, V>>>,
    ttl: std::time::Duration,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> Cache<K, V> {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            store: Arc::new(RwLock::new(std::collections::HashMap::new())),
            ttl: std::time::Duration::from_secs(ttl_seconds),
        }
    }
    
    /// 获取值，如果不存在返回 None
    pub async fn get(&self, key: &K) -> Option<V> {
        let store = self.store.read().await;
        store.get(key).cloned()
    }
    
    /// 插入值
    pub async fn set(&self, key: K, value: V) {
        let mut store = self.store.write().await;
        store.insert(key, value);
    }
}
```

### SQL（数据库查询）

```sql
-- 查询过去30天每个类别的销售总额
WITH monthly_sales AS (
    SELECT 
        p.category_id,
        c.name AS category_name,
        SUM(oi.quantity * oi.unit_price) AS total_revenue,
        COUNT(DISTINCT o.id) AS order_count
    FROM orders o
    JOIN order_items oi ON o.id = oi.order_id
    JOIN products p ON oi.product_id = p.id
    JOIN categories c ON p.category_id = c.id
    WHERE o.created_at >= CURRENT_DATE - INTERVAL '30 days'
        AND o.status = 'completed'
    GROUP BY p.category_id, c.name
)
SELECT 
    category_name,
    total_revenue,
    order_count,
    ROUND(total_revenue / NULLIF(order_count, 0), 2) AS avg_order_value
FROM monthly_sales
ORDER BY total_revenue DESC;
```

---

## 4. 表格展示

### 基础对齐

| 算法 | 时间复杂度 | 空间复杂度 | 稳定性 |
|:---:|:---:|:---:|:---:|
| 快速排序 | O(n \log n) | O(\log n) | ❌ 不稳定 |
| 归并排序 | O(n \log n) | O(n) | ✅ 稳定 |
| 堆排序 | O(n \log n) | O(1) | ❌ 不稳定 |
| 计数排序 | O(n + k) | O(k) | ✅ 稳定 |

### 系统性能指标

| 服务 | QPS | 延迟 P_{99} | 错误率 | 状态 |
|----|:---:|:---:|:---:|:---:|
| User Service | 12,000 | 25ms | 0.01% | 🟢 健康 |
| Order Service | 8,500 | 45ms | 0.05% | 🟢 健康 |
| Payment Service | 3,200 | 120ms | 0.2% | 🟡 警告 |
| Notification | 15,000 | 15ms | 0.001% | 🟢 健康 |

---

## 5. 其他 GFM 特性

### 任务列表
- [x] 完成架构设计文档
- [x] 实现用户认证模块 HMAC_{SHA256}
- [ ] 优化数据库查询（目标：减少 N+1 问题）
- [ ] 集成支付网关
- [ ] 编写单元测试（覆盖率 \geq 80\%）

### 折叠详情
<details>
<summary>📊 性能测试详细数据（点击展开）</summary>

**测试环境**
- CPU: Intel Xeon E5-2680 v4 @ 2.40GHz
- 内存: 32GB DDR4
- 网络: 10Gbps 内网

**测试结果**
```json
{
  "concurrent_users": 10000,
  "total_requests": 1000000,
  "avg_response_time": "12.5ms",
  "error_rate": "0.001%",
  "throughput": "45000 req/s"
}
```
</details>

### 脚注引用
Markdown 支持多种数学表达式的渲染方式[^1]，包括 MathJax 和 KaTeX。Mermaid 图表在 GitHub 原生支持[^2]，无需额外插件。

[^1]: 数学公式使用 LaTeX 语法，行内用 `...`，块级用 `...`。
[^2]: Mermaid 语法参考官方文档：https://mermaid.js.org/

---

## 6. 提示块（Alerts）

> [!NOTE]
> 这是普通说明块，用于补充信息 E = mc^2。

> [!TIP]
> 提示：使用缓存可以显著降低数据库查询复杂度，从 O(n) 降至 O(1)。

> [!IMPORTANT]
> 重要：生产环境务必开启 HTTPS 和 JWT 签名验证。

> [!WARNING]
> 警告：直接拼接 SQL 会导致注入漏洞，
