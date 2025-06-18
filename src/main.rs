mod job;
mod printer;
mod osim;

use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use std::sync::Arc;
use chrono::Utc;
use osim::SPOOLing::{rawJob, SPOOLing};
use crate::printer::Printer;

#[derive(serde::Deserialize)]
struct PrintRequest {
    priority: u32,
    team_name: String,
    file_content: String,
    color: bool,
}

struct AppState {
    spooling: Arc<SPOOLing>
}

async fn submit_job(
    data: web::Data<AppState>,
    req: web::Json<PrintRequest>,
) -> impl Responder {
    // 构造 rawJob，为防止所有权问题，使用 to_string() 代替 clone()
    let raw_job = rawJob {
        priority: req.priority,
        team_name: req.team_name.to_string(),
        submit_time: Utc::now(),
        file_content: req.file_content.to_string(),
        color: req.color,
    };

    // 提交到 SPOOLing 系统
    match data.spooling.submit_job(raw_job) {
        Ok(job_id) => HttpResponse::Ok().body(format!("打印任务提交成功，任务ID: {}", job_id)),
        Err(e) => HttpResponse::ServiceUnavailable().body(format!("提交失败: {}", e)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 创建打印机和 SPOOLing 系统
    let printer = Arc::new(Printer::new());
    let spooling = Arc::new(SPOOLing::new(10, 10, 10, 10));
    
    // 启动 SPOOLing 工作线程
    spooling.clone().start_workers(printer);

    let app_state = web::Data::new(AppState {
        spooling: spooling.clone(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/print", web::post().to(submit_job))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}