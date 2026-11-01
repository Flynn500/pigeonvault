pub struct QueueConfig {
    pub segment_bytes: u64,
    pub max_bytes: u64,
    pub fsync: FsyncPolicy,
    pub on_full: FullPolicy,
}

pub struct FsyncPolicy { }

pub struct FullPolicy { }