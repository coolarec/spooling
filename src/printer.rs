use crate::job::{Job};

use std::thread;
use std::time::Duration;

use genpdf::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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

    pub fn set_status(&mut self, new_status: PrinterStatus) {
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
        
        //模拟打印，一份文件等待10s
        thread::sleep(Duration::from_secs(10));

        Ok(())
    }

    pub fn submit_task(self: &Arc<Self>, job: Job) -> Result<usize, ()> {
        let prev_status = self.status.compare_exchange(
            PrinterStatus::Free as usize,
            PrinterStatus::Printing as usize,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );

        if prev_status.is_err() {
            return Err(());
        }

        let job_id = job.job_id;
        let job_clone = job.clone();

        let printer_arc = Arc::clone(self);
        thread::spawn(move || {
            let mut res = String::new();
            for (count, line) in job_clone.file_content.lines().enumerate() {
                res += &format!("{:>3}: {}\n", count + 1, line);
            }
            let _ = printer_arc.print_file(&res, &job_clone.file_name);
            printer_arc.status.store(PrinterStatus::Free as usize, Ordering::SeqCst);
            printer_arc.printed_count.fetch_add(1, Ordering::SeqCst);
        });

        Ok(job_id)
    }
}