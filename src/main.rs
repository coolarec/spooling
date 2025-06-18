mod job;
mod printer;

use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use std::sync::Arc;
use job::Job;
use printer::Printer;
use chrono::Utc;


struct AppState {
    printer: Arc<printer::Printer>,
}

#[derive(serde::Deserialize)]
struct PrintRequest {
    priority: u32,
    team_name: String,
    file_content: String,
    color: bool,
}


async fn print_job(
    data: web::Data<AppState>,
    req: web::Json<PrintRequest>,
) -> impl Responder {
    let job = Job::new(
        req.priority,
        req.team_name.clone(),
        Utc::now(),
        req.file_content.clone(),
        req.color,
    );
    match data.printer.submit_task(job) {
        Ok(job_id) => HttpResponse::Ok().body(format!("打印任务提交成功，任务ID: {}", job_id)),
        Err(_) => HttpResponse::Conflict().body("打印机正忙，请稍后再试"),
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let printer = Arc::new(Printer::new());
    let app_state = web::Data::new(AppState {
        printer: printer.clone(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/print", web::post().to(print_job))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}