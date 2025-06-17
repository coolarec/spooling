use crate::job::{Job, JobStatus};

use chrono::Datelike;
use chrono::Timelike;

use genpdf::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, PartialEq, Eq)]
pub enum PrinterStatus {
    Free=0,   // 等待打印
    Printing=1,  // 正在打印
}

//记录打印机状态
pub struct Printer {
    status:AtomicUsize,
    printed_count:AtomicUsize,
}

impl Printer{
    
    //初始时打印机为空，可以打印东西
    pub fn new()->Self{
        Self{
            status: AtomicUsize::new(PrinterStatus::Free as usize),
            printed_count:AtomicUsize::new(0),
        }
    }

    //读取打印机工作状态
    pub fn get_status(&self) -> PrinterStatus {
        match self.status.load(Ordering::SeqCst) {
            0 => PrinterStatus::Free,
            1 => PrinterStatus::Printing,
            _ => panic!("Unknown printer status"),
        }
    }

    pub fn set_status(&self, new_status: PrinterStatus) {
        self.status.store(new_status as usize, Ordering::SeqCst);
    }

    //模拟打印功能
    fn print_file(&self,code: &str,file_name:&str)->Result<(),()> {
        //加载字体
        let font_family =
            fonts::from_files("./fonts", "MapleMono", None).expect("Failed to load font family");

        let mut doc = Document::new(font_family);
        doc.set_title("Demo document");
        
        //设置页面样式
        let mut decorator = genpdf::SimplePageDecorator::new();
        decorator.set_margins(10);
        doc.set_page_decorator(decorator);


        for line in code.lines() {
            let p = genpdf::elements::Paragraph::new(line);
            doc.push(p);
        }

        doc.render_to_file(&format!("{}.pdf", file_name))
            .expect("Failed to write PDF file");
        Ok(())
    }

    pub fn submit_task(&self, job: Job) -> Result<(), ()> {
        // 先尝试把状态从 Free(0) 改成 Printing(1)，如果当前不是Free就返回Err
        let prev_status = self.status.compare_exchange(
            PrinterStatus::Free as usize,
            PrinterStatus::Printing as usize,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );

        if prev_status.is_err() {
            // 当前状态不是Free，打印机忙，拒绝执行
            return Err(());
        }

        // 状态切换成功，执行打印
        let mut res = String::new();
        for (count, line) in job.file_content.lines().enumerate() {
            res += &format!("{:>3}: {}\n", count + 1, line);
        }
        
        self.print_file(&res, &job.file_name);

        // 打印完成，状态重置为 Free
        self.status.store(PrinterStatus::Free as usize, Ordering::SeqCst);

        // 计数 +1
        self.printed_count.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }
}