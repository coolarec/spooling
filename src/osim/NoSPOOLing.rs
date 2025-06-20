use crate::job::{Job,JobStatus};
use crate::printer::{Printer,PrinterStatus};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::osim::SPOOLing::rawJob;

#[derive(Clone)]
pub struct NoSPOOLing {
    pub status_map: Arc<Mutex<HashMap<u64, Job>>>,
    printer:Arc<Printer>,
}
impl NoSPOOLing {
    pub fn new(printer:Arc<Printer>) -> Self {
        Self {
            status_map: Arc::new( Mutex::new(HashMap::new())),
            printer:printer,
        }
    }
    pub fn get_status(&self)->String{
        let status=self.printer.clone().get_status();
        match status {
            PrinterStatus::Free=>{
                format!("OK")
            }
            PrinterStatus::Printing=>{
                format!("OK")
            }
        }
    }
    pub fn submit_job(&self, data: rawJob) -> Result<usize, String> {
        // 创建新的 Job
        let mut job = Job::new(
            data.priority,
            data.team_name,
            data.submit_time,
            data.file_content,
            data.color,
            data.problem_name,
        );

        let job_id = job.clone().job_id;
        job.status = JobStatus::Waiting;
        let mut status_map = self.status_map.clone();
        status_map
            .lock()
            .unwrap()
            .insert(job.job_id as u64, job.clone());

        // 尝试推入输入缓冲区
        match self.printer.submit_task(job.clone()) {
            Ok(_) => {
                let job_id = job.job_id.clone();
                job.status = JobStatus::Completed;
                let mut status_map = self.status_map.clone();
                status_map
                    .lock()
                    .unwrap()
                    .insert(job.clone().job_id as u64, job.clone());
                println!("任务 {} 已开始打印", job_id);

                Ok(job_id)
            }
            Err(mut job) => {
                println!("任务 {} 提交失败", job_id);

                job.status = crate::job::JobStatus::SubmitFailed;
                status_map
                    .lock()
                    .unwrap()
                    .insert(job.job_id as u64, job.clone());

                Err("缓冲区已满".to_string())
            }
        }
    }
    /// 获取所有提交成功的任务id
    pub fn get_active_job_id(&self)-> Vec<u64>{
        let status_map = self.status_map.lock().unwrap();
        status_map
            .iter()
            .filter_map(|(&job_id, job)| {
                if job.status != JobStatus::SubmitFailed {
                    Some(job_id)
                } else {
                    None
                }
            })
            .collect()
    }

}
