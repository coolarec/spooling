use crate::job::{Job, JobStatus};
use crate::printer::{Printer, PrinterStatus};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::cmp::Reverse; // 用于反转比较实现小根堆
use std::collections::BinaryHeap;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::{thread, time::Duration};

pub struct rawJob {
    pub priority: u32,
    pub team_name: String,
    pub submit_time: DateTime<Utc>,
    pub file_content: String,
    pub color: bool,
}

// 实现 PartialEq 和 Eq
impl PartialEq for rawJob {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.submit_time == other.submit_time
    }
}

impl Eq for rawJob {}

// 实现 PartialOrd 和 Ord
impl PartialOrd for rawJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for rawJob {
    fn cmp(&self, other: &Self) -> Ordering {
        // 先比较优先级（注意：priority越小优先级越高）
        match self.priority.cmp(&other.priority) {
            // 如果优先级相同，再比较提交时间（越早的优先级越高）
            Ordering::Equal => other.submit_time.cmp(&self.submit_time),
            // 如果优先级不同，直接返回优先级比较结果
            ordering => ordering,
        }
    }
}
#[derive(Clone)]
pub struct Buffer<T> {
    queue: Arc<Mutex<VecDeque<T>>>,
    max_size: usize,
    ready: Arc<Condvar>,
    name: String,
}

impl<T> Buffer<T> {
    pub fn new(name: &str, max_size: usize) -> Self {
        Buffer {
            queue: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
            ready: Arc::new(Condvar::new()),
            name: name.to_string(),
        }
    }

    // 非阻塞尝试添加
    pub fn try_push(&self, item: T) -> Result<(), T> {
        let mut queue = self.queue.lock().unwrap();
        if queue.len() < self.max_size {
            queue.push_back(item);
            self.ready.notify_one();
            Ok(())
        } else {
            Err(item)
        }
    }

    // 阻塞添加
    pub fn push(&self, item: T) {
        let mut queue = self.queue.lock().unwrap();
        while queue.len() >= self.max_size {
            queue = self.ready.wait(queue).unwrap();
        }
        queue.push_back(item);
        self.ready.notify_one();
    }

    // 非阻塞尝试取出
    pub fn try_pop(&self) -> Option<T> {
        let mut queue = self.queue.lock().unwrap();
        queue.pop_front()
    }

    // 阻塞取出
    pub fn pop(&self) -> T {
        // print!("1-------------\n");
        let mut queue = self.queue.lock().unwrap();
        // print!("2-------------\n");
        while queue.is_empty() {
            // print!("3-------------\n");
            queue = self.ready.wait(queue).unwrap();
            // print!("4-------------\n");
        }
        // print!("5-------------\n");
        queue.pop_front().unwrap()
    }

    // 获取当前大小
    pub fn size(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    // 获取容量
    pub fn capacity(&self) -> usize {
        self.max_size
    }
}

/// 基于小根堆实现的井结构  
#[derive(Clone)]
pub struct HeapWell<T: Ord> {
    heap: Arc<Mutex<BinaryHeap<Reverse<T>>>>, // 使用Reverse实现小根堆
    ready: Arc<Condvar>,
    max_size: usize,
    name: String,
}

impl<T: Ord> HeapWell<T> {
    pub fn new(name: &str, max_size: usize) -> Self {
        HeapWell {
            heap: Arc::new(Mutex::new(BinaryHeap::with_capacity(max_size))),
            ready: Arc::new(Condvar::new()),
            max_size,
            name: name.to_string(),
        }
    }

    /// 插入元素（按小根堆排序）
    pub fn push(&self, item: T) -> Result<(), T> {
        let mut heap = self.heap.lock().unwrap();
        if heap.len() >= self.max_size {
            return Err(item);
        }
        heap.push(Reverse(item));
        self.ready.notify_one();
        Ok(())
    }

    /// 阻塞插入
    pub fn push_blocking(&self, item: T) {
        let mut heap = self.heap.lock().unwrap();
        while heap.len() >= self.max_size {
            heap = self.ready.wait(heap).unwrap();
        }
        heap.push(Reverse(item));
        self.ready.notify_one();
    }

    /// 弹出最小元素
    pub fn pop(&self) -> Option<T> {
        let mut heap = self.heap.lock().unwrap();
        heap.pop().map(|Reverse(item)| item)
    }

    /// 阻塞弹出
    pub fn pop_blocking(&self) -> T {
        let mut heap = self.heap.lock().unwrap();
        while heap.is_empty() {
            heap = self.ready.wait(heap).unwrap();
        }
        heap.pop().unwrap().0
    }

    /// 查看最小元素但不移除
    pub fn peek(&self) -> Option<T>
    where
        T: Clone,
    {
        let heap = self.heap.lock().unwrap();
        heap.peek().map(|Reverse(item)| item.clone())
    }

    /// 当前大小
    pub fn len(&self) -> usize {
        self.heap.lock().unwrap().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.heap.lock().unwrap().is_empty()
    }
}

pub struct SPOOLing {
    input_buffer: Buffer<Job>,
    input_well: HeapWell<Job>,
    output_well: HeapWell<Job>,
    output_buffer: Buffer<Job>,
    status_map: Arc<Mutex<HashMap<u64, Job>>>,
}

impl SPOOLing {
    pub fn new(
        input_buffer_size: usize,
        input_well_size: usize,
        output_well_size: usize,
        output_buffer_size: usize,
    ) -> Self {
        SPOOLing {
            input_buffer: Buffer::new("input_buffer", input_buffer_size),
            input_well: HeapWell::new("input_well", input_well_size),
            output_well: HeapWell::new("output_well", output_well_size),
            output_buffer: Buffer::new("output_buffer", output_buffer_size),
            status_map: Arc::new(Mutex::new(HashMap::new())),
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
        );

        let job_id = job.job_id;
        job.status = JobStatus::Waiting;
        let status_map = self.status_map.clone();
        status_map
            .lock()
            .unwrap()
            .insert(job.job_id as u64, job.clone());

        // 尝试推入输入缓冲区
        match self.input_buffer.try_push(job) {
            Ok(_) => {
                println!("任务 {} 已提交到输入缓冲区", job_id);
                Ok(job_id)
            }
            Err(mut job) => {
                println!("缓冲区已满，任务 {} 提交失败", job_id);

                job.status = crate::job::JobStatus::SubmitFailed;
                status_map
                    .lock()
                    .unwrap()
                    .insert(job.job_id as u64, job.clone());

                Err("缓冲区已满".to_string())
            }
        }
    }

    pub fn start_workers(&self, printer: Arc<Printer>) {
        // 输入缓冲区 → 输入井
        {
            let input_buffer = self.input_buffer.clone();
            let input_well = self.input_well.clone();
            thread::spawn(move || {
                loop {
                    let job = input_buffer.pop(); // 阻塞
                    println!("[INFO] 输入缓冲区弹出 Job {}，准备放入输入井", job.job_id);
                    input_well.push_blocking(job);
                    println!("[INFO] Job 已成功进入输入井");
                }
            });
        }

        // 输入井 → 输出井
        {
            let input_well = self.input_well.clone();
            let output_well = self.output_well.clone();
            let status_map = self.status_map.clone();
            thread::spawn(move || {
                loop {
                    let mut job = input_well.pop_blocking(); // 阻塞
                    println!("[INFO] 输入井中取出 Job {}，准备格式化内容", job.job_id);

                    let formatted = format!(
                        "\\\\ team_name: {}\n\\\\ submit_time: {}\n\n{}",
                        job.team_name, job.submit_time, job.file_content
                    );
                    job.file_content = formatted;

                    println!("[INFO] Job {} 格式化完成，状态写入状态表", job.job_id);
                    status_map
                        .lock()
                        .unwrap()
                        .insert(job.job_id as u64, job.clone());

                    let id = job.job_id;
                    output_well.push_blocking(job);
                    println!("[INFO] Job {} 推入输出井", id);
                }
            });
        }

        // 输出井 → 输出缓冲区
        {
            let output_well = self.output_well.clone();
            let output_buffer = self.output_buffer.clone();
            thread::spawn(move || {
                loop {
                    let job = output_well.pop_blocking(); // 阻塞
                    println!("[INFO] 输出井中弹出 Job {}，推入输出缓冲区", job.job_id);
                    output_buffer.push(job);
                }
            });
        }

        // 输出缓冲区 → 打印机
        {
            let output_buffer = self.output_buffer.clone();
            let printer_arc = printer.clone();
            let status_map = self.status_map.clone();

            thread::spawn(move || {
                loop {
                    print!("push---------\n");
                    let job = output_buffer.pop(); // 阻塞
                    print!("push down---------\n");
                    let job_id = job.job_id;
                    let mut job_clone = job.clone();
                    let printer_clone = printer_arc.clone();
                    let status_map = status_map.clone();

                    println!("[INFO] 打印线程启动：Job {}", job_id);
                    if printer_clone.submit_task(job_clone.clone()).is_ok() {
                        job_clone.complete();
                        println!("[SUCCESS] Job {} 打印成功，状态更新为已完成", job_id);
                    } else {
                        job_clone.status = JobStatus::SubmitFailed;
                        println!("[ERROR] Job {} 打印失败，状态更新为失败", job_id);
                    }
                }
            });
        }
    }
}
