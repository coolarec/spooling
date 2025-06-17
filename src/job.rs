use std::cmp::Ordering;
use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use serde::{Serialize, Deserialize};

static TOTAL_TASKS: AtomicUsize = AtomicUsize::new(0); //总提交任务数量
static COMPLETED_TASKS: AtomicUsize = AtomicUsize::new(0); //成功任务数量

//返回总任务和
pub fn stats() -> (usize, usize) {
    (
        TOTAL_TASKS.load(AtomicOrdering::SeqCst),
        COMPLETED_TASKS.load(AtomicOrdering::SeqCst),
    )
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Waiting,      // 等待打印
    Printing,     // 正在打印
    Completed,    // 打印完成
    SubmitFailed, // 提交失败
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub print_id: u32,
    pub file_name: String,
    pub submit_time: DateTime<Utc>,
    pub file_content: String,
    pub color: bool,
    pub status: JobStatus,
    pub start_print_time: Option<DateTime<Utc>>, // 打印开始时间
    pub end_print_time: Option<DateTime<Utc>>,   // 打印结束时间
}

impl Job {
    pub fn new(
        print_id: u32,
        file_name: String,
        submit_time: DateTime<Utc>,
        file_content: String,
        color: bool,
    ) -> Self {
        TOTAL_TASKS.fetch_add(1, AtomicOrdering::SeqCst);
        Self {
            print_id,
            file_name,
            submit_time,
            file_content,
            color,
            status: JobStatus::Waiting,
            start_print_time: None,
            end_print_time: None,
        }
    }

    pub fn start_printing(&mut self) {
        self.status = JobStatus::Printing;
        self.start_print_time=Some(Utc::now());
    }

    pub fn complete(&mut self) {
        if self.status != JobStatus::Completed {
            self.status = JobStatus::Completed;
            self.end_print_time = Some(Utc::now());
            COMPLETED_TASKS.fetch_add(1, AtomicOrdering::SeqCst);
        }
    }

    pub fn display(&self) {
        println!(
            "任务ID: {}, 文件: {}, 提交时间: {}, 彩色: {}, 状态: {:?}",
            self.print_id,
            self.file_name,
            self.submit_time.format("%Y-%m-%d %H:%M:%S"),
            self.color,
            self.status
        );
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.print_id == other.print_id && self.submit_time == other.submit_time
    }
}
impl Eq for Job {}

impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.print_id.cmp(&other.print_id) {
            Ordering::Equal => self.submit_time.cmp(&other.submit_time),
            other_order => other_order,
        }
    }
}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}