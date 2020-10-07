//!
//! # Env Request
//!

pub mod add;
pub mod del;
pub mod get;
pub mod list;
pub mod listall;
pub mod push;
pub mod run;
pub mod show;
pub mod start;
pub mod stop;
pub mod update;

pub use add::*;
pub use del::*;
pub use get::*;
pub use list::*;
pub use listall::*;
pub use push::*;
pub use run::*;
pub use show::*;
pub use start::*;
pub use stop::*;
pub use update::*;

use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

lazy_static! {
    // 传输文件时并发太多没有意义,
    // 使用小规模的线程池限制并发数量
    static ref POOL: Pool = Pool::new(3);
}

struct Pool {
    inner: Arc<Mutex<ThreadPool>>,
}

impl Pool {
    #[inline(always)]
    fn new(n: usize) -> Pool {
        Pool {
            inner: Arc::new(Mutex::new(ThreadPool::new(n))),
        }
    }

    #[inline(always)]
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.inner.lock().unwrap().execute(job)
    }
}
