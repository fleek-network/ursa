use super::Cache;

pub trait AdminCache: Send + Sync + 'static {
    fn purge(&mut self);
}

impl AdminCache for Cache {
    fn purge(&mut self) {
        self.tlrfu.purge();
    }
}
