// src/lib.rs
use dashmap::DashMap;
use serde_json;
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

impl StackStore {
    /// 创建新的空StackStore实例
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    /// 将值压入指定键对应的栈顶
    ///
    /// # 注意
    /// - 如果键不存在会自动创建空栈
    /// - 可能因互斥锁污染导致panic（当持有锁的线程发生panic时）
    pub fn set(&self, key: &str, value: String) {
        self.inner
            .entry(key.to_string())
            .or_insert_with(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap()
            .push(value);
    }

    /// 从指定键对应的栈顶弹出值
    ///
    /// # 返回值
    /// - Some(String): 当栈存在且非空时返回栈顶值
    /// - None: 当键不存在或栈为空时返回
    ///
    /// # 注意
    /// 可能因互斥锁污染导致panic
    pub fn get(&self, key: &str) -> Option<String> {
        self.inner.get(key).and_then(|stack| {
            let guard = stack.lock().unwrap();
            serde_json::to_string(&*guard).ok()
        })
    }

    /// 获取过滤后的栈快照
    ///
    /// # 参数
    /// - key_filter: 键名包含的过滤字符串
    ///
    /// # 返回值
    /// 包含(键名, 栈克隆)的向量，按LIFO顺序保持元素
    ///
    /// # 注意
    /// 获取时会克隆整个栈内容，可能因互斥锁污染导致panic
    pub fn iter(&self, key_filter: &str) -> Option<String> {
        let r: Vec<(String, Vec<String>)> = self
            .inner
            .iter()
            .filter(|entry| entry.key().contains(key_filter))
            .map(|entry| {
                let stack = entry.value().lock().unwrap();
                (entry.key().clone(), stack.clone())
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
    use std::thread;

    #[test]
    fn test_set_and_get() {
        let store = GLOBAL_STACK_STORE.clone();
        store.set("test", "value1".into());
        store.set("test", "value2".into());

        let value = store.get("test").unwrap();
        assert_eq!(value, r#"["value1","value2"]"#);
    }

    #[test]
    fn test_get_non_existent_key() {
        let store = GLOBAL_STACK_STORE.clone();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_iter_filter() {
        let store = GLOBAL_STACK_STORE.clone();
        store.set("apple1", "fruit".into());
        store.set("banana2", "fruit".into());
        store.set("carrot3", "vegetable".into());

        let results = store.iter("na");
        assert_eq!(results, Some(r#"[["banana2",["fruit"]]]"#.to_string()));
    }

    #[test]
    fn test_del_stack() {
        let store = GLOBAL_STACK_STORE.clone();
        store.set("temp", "data".into());
        assert!(store.del_stack("temp"));
        assert!(!store.del_stack("temp"));
    }

    #[test]
    fn test_concurrent_access() {
        let store = Arc::new(StackStore::new());

        for i in 0..10 {
            let store = store.clone();
            store.set("counter", i.to_string());
        }

        let value = store.get("counter").unwrap();
        assert_eq!(value, r#"["0","1","2","3","4","5","6","7","8","9"]"#);
    }
}
