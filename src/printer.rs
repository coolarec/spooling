
#[derive(Debug, PartialEq, Eq)]
pub enum PrinterStatus {
    Free,   // 等待打印
    Printing,  // 正在打印
}

//记录打印机状态
pub struct Printer {
    status:PrinterStatus,
}

impl Job{
    
    //初始时打印机为空，可以打印东西
    pub fn new()->Self{
        Self{
            status:Free,
        }
    }

    //模拟演示
    fn print_file(file:String)->bool{

    }
}