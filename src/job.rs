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
///  四种工作状态
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize,Clone)]
pub enum JobStatus {
    Waiting,      // 等待打印
    Printing,     // 正在打印
    Completed,    // 打印完成
    SubmitFailed, // 提交失败
}




#[derive(Debug, Serialize, Deserialize,Clone)]
pub struct Job {
    pub job_id: usize,
    pub priority:u32,
    pub team_name:String,
    pub file_name: String,
    pub problem_name:String,
    pub submit_time: DateTime<Utc>,
    pub file_content: String,
    pub color: bool,
    pub status: JobStatus,
    pub start_print_time: Option<DateTime<Utc>>, // 打印开始时间
    pub end_print_time: Option<DateTime<Utc>>,   // 打印结束时间
}

impl Job {
    pub fn new(
        priority: u32,
        team_name:String,
        submit_time: DateTime<Utc>,
        file_content: String,
        color: bool,
        problem_name:String,
    ) -> Self {
        let job_id=TOTAL_TASKS.fetch_add(1, AtomicOrdering::SeqCst);
        
        let timestamp = submit_time.format("%Y%m%d_%H%M%S").to_string();
        let file_name = format!("{}_{}_{}", team_name, timestamp, job_id);
        
        Self {
            job_id,
            priority,
            team_name,
            file_name,
            problem_name,
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
            self.job_id,
            self.file_name,
            self.submit_time.format("%Y-%m-%d %H:%M:%S"),
            self.color,
            self.status
        );
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.submit_time == other.submit_time
    }
}

impl Eq for Job {}

impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority.cmp(&other.priority) {
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