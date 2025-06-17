use genpdf::*;

#[derive(Debug, PartialEq, Eq)]
pub enum PrinterStatus {
    Free,   // 等待打印
    Printing,  // 正在打印
}

//记录打印机状态
pub struct Printer {
    status:PrinterStatus,
}

impl Printer{
    
    //初始时打印机为空，可以打印东西
    pub fn new()->Self{
        Self{
            status:PrinterStatus::Free,
        }
    }

    //模拟打印功能
    pub fn print_file(&self,code: &str,file_name:&str) {
        //加载字体
        let font_family =
            fonts::from_files("./fonts", "MapleMono", None).expect("Failed to load font family");

        let mut doc = Document::new(font_family);
        doc.set_title("Demo document");
        let mut decorator = genpdf::SimplePageDecorator::new();
        decorator.set_margins(10);
        doc.set_page_decorator(decorator);

        for line in code.lines() {
            let p = genpdf::elements::Paragraph::new(line);
            doc.push(p);
        }

        doc.render_to_file(&format!("{}.pdf", file_name))
            .expect("Failed to write PDF file");
    }


}