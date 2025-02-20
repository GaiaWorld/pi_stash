# StackStore

一个线程安全的栈式存储结构，支持并发访问和键值管理。

## 概述

StackStore 提供基于字符串键的线程安全栈存储，支持以下特性：
- **线程安全**：使用 `DashMap` 和 `Mutex` 实现高效并发访问
- **栈操作**：支持按键压入(push)/获取(get)数据
- **过滤查询**：通过子字符串匹配检索键值
- **栈删除**：支持整栈删除操作

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
stackstore = { git = "https://github.com/your-repo-url" }  # 替换为实际仓库地址
dashmap = "5.4.0"
serde_json = "1.0"
```

## 使用示例

### 基础操作
```rust
use stackstore::StackStore;

let store = StackStore::new();

// 压入数据
store.set("fruit", "apple".into());
store.set("fruit", "banana".into());

// 获取整个栈的JSON序列化结果
assert_eq!(store.get("fruit").unwrap(), r#"["apple","banana"]"#);

// 删除栈
assert!(store.del_stack("fruit"));
```

### 过滤查询
```rust
store.set("server:1", "online".into());
store.set("server:2", "offline".into());
store.set("client:3", "active".into());

// 查询包含"server"的键
let results = store.iter("server");
assert_eq!(results.len(), 2);
```

## API参考

### `StackStore::new()`
创建新的空存储实例。

### `set(key: &str, value: String)`
- 将值压入指定键对应的栈顶
- 自动为不存在的键创建新栈

### `get(key: &str) -> Option<String>`
- 返回整个栈的JSON序列化字符串
- 返回 `None` 当键不存在或栈为空

### `iter(key_filter: &str) -> Vec<(String, Vec<String>)>`
- 返回包含过滤字符串的键及其栈克隆
- 结果按后进先出(LIFO)顺序保持

### `del_stack(key: &str) -> bool`
- 成功删除返回 `true`，键不存在返回 `false`

## 注意事项

1. **锁安全**：方法可能因互斥锁污染导致panic（当持有锁的线程panic时）
2. **序列化**：`get` 方法返回使用 `serde_json` 序列化的JSON数组字符串
3. **克隆开销**：`iter` 方法会克隆整个栈内容，注意性能影响

## 运行测试

```bash
cargo test --lib
```

测试包括：
- 基础功能测试
- 并发访问测试
- 过滤查询测试
- 删除操作测试

## 许可证

MIT License