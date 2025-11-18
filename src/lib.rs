// src/lib.rs
use dashmap::DashMap;
use std::sync::{Arc, Mutex};

#[macro_use]
extern crate lazy_static;

lazy_static! {
    // 全局存储js堆栈信息
    pub static ref GLOBAL_STACK_STORE: Arc<StackStore> = Arc::new(StackStore::new());
}

/// 线程安全的栈式存储结构，使用字符串作为键，支持并发访问
///
/// 使用 DashMap 管理键值对，每个键对应一个受互斥锁(Mutex)保护的字符串栈
pub struct StackStore {
    inner: DashMap<String, Mutex<Vec<String>>>,
}

impl Default for StackStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StackStore {
    /// 创建新的空StackStore实例
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    /// 将值压入指定键对应的栈顶
    ///
    /// # 参数
    /// - key: 栈的键名
    /// - value: 要压入的值
    ///
    /// # 注意
    /// - 如果键不存在会自动创建空栈
    /// - 在极少数情况下可能因互斥锁污染导致panic（当持有锁的线程发生panic时）
    pub fn set(&self, key: &str, value: String) {
        // 首先尝试获取现有条目
        if let Some(stack) = self.inner.get(key) {
            match stack.lock() {
                Ok(mut guard) => {
                    guard.push(value);
                    return;
                }
                Err(poisoned) => {
                    let mut guard = poisoned.into_inner();
                    guard.push(value);
                    return;
                }
            }
        }

        // 如果键不存在，创建新的条目
        let vec = vec![value];
        self.inner.insert(key.to_string(), Mutex::new(vec));
    }

    /// 获取指定键对应的整个栈的JSON序列化字符串
    ///
    /// # 参数
    /// - key: 栈的键名
    ///
    /// # 返回值
    /// - Some(String): 包含整个栈的JSON数组字符串
    /// - None: 当键不存在时返回
    ///
    /// # 注意
    /// 在极少数情况下可能因互斥锁污染导致panic，但会尝试恢复数据
    pub fn get(&self, key: &str) -> Option<String> {
        self.inner.get(key).and_then(|stack| {
            match stack.lock() {
                Ok(guard) => serde_json::to_string(&*guard).ok(),
                Err(poisoned) => {
                    // 从被污染的锁中恢复数据
                    let guard = poisoned.into_inner();
                    serde_json::to_string(&*guard).ok()
                }
            }
        })
    }

    /// 获取过滤后的栈快照
    ///
    /// # 参数
    /// - key_filter: 键名包含的过滤字符串
    ///
    /// # 返回值
    /// - Some(String): 包含过滤结果的JSON数组字符串，每个元素是[键名, 栈内容数组]
    /// - None: 当序列化失败时返回
    ///
    /// # 注意
    /// - 获取时会克隆整个栈内容，可能影响性能
    /// - 在极少数情况下可能因互斥锁污染导致panic，但会尝试恢复数据
    pub fn iter(&self, key_filter: &str) -> Option<String> {
        let r: Vec<(String, Vec<String>)> = self
            .inner
            .iter()
            .filter(|entry| entry.key().contains(key_filter))
            .map(|entry| {
                match entry.value().lock() {
                    Ok(stack) => (entry.key().clone(), stack.clone()),
                    Err(poisoned) => {
                        // 从被污染的锁中恢复数据
                        let stack = poisoned.into_inner();
                        (entry.key().clone(), stack.clone())
                    }
                }
            })
            .collect();
        serde_json::to_string(&r).ok()
    }

    /// 删除指定键对应的整个栈
    ///
    /// # 返回值
    /// - true: 成功删除存在的键
    /// - false: 键不存在
    pub fn del_stack(&self, key: &str) -> bool {
        self.inner.remove(key).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let store = StackStore::new();
        store.set("test", "value1".into());
        store.set("test", "value2".into());

        let value = store.get("test").unwrap();
        assert_eq!(value, r#"["value1","value2"]"#);
    }

    #[test]
    fn test_get_non_existent_key() {
        let store = StackStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_iter_filter() {
        let store = StackStore::new();
        store.set("apple1", "fruit".into());
        store.set("banana2", "fruit".into());
        store.set("carrot3", "vegetable".into());

        let results = store.iter("na");
        assert_eq!(results, Some(r#"[["banana2",["fruit"]]]"#.to_string()));
    }

    #[test]
    fn test_del_stack() {
        let store = StackStore::new();
        store.set("temp", "data".into());
        assert!(store.del_stack("temp"));
        assert!(!store.del_stack("temp"));
    }

    #[test]
    fn test_concurrent_access() {
        let store = Arc::new(StackStore::new());

        // 创建多个线程同时写入数据
        let mut handles = vec![];
        for i in 0..10 {
            let store = store.clone();
            let handle = std::thread::spawn(move || {
                store.set("counter", i.to_string());
            });
            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }

        let value = store.get("counter").unwrap();
        // 验证所有值都被正确添加
        assert!(value.contains("\"0\"") && value.contains("\"9\""));
    }

    #[test]
    fn test_empty_stack() {
        let store = StackStore::new();
        // 测试空栈的序列化
        store.set("empty", "".into());
        let result = store.get("empty");
        assert_eq!(result, Some(r#"[""]"#.to_string()));
    }

    #[test]
    fn test_multiple_stacks() {
        let store = StackStore::new();
        store.set("stack1", "a".into());
        store.set("stack1", "b".into());
        store.set("stack2", "x".into());
        store.set("stack2", "y".into());

        let stack1 = store.get("stack1").unwrap();
        let stack2 = store.get("stack2").unwrap();

        assert_eq!(stack1, r#"["a","b"]"#);
        assert_eq!(stack2, r#"["x","y"]"#);
    }
}
