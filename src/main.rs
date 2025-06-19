mod job;
mod printer;
mod osim;

use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use std::sync::Arc;
use chrono::Utc;
use osim::SPOOLing::{rawJob, SPOOLing};
use osim::NoSPOOLing::NoSPOOLing;
use printer::Printer;
use serde_json::json;

#[derive(serde::Deserialize)]
struct PrintRequest {
    priority: u32,
    team_name: String,
    file_content: String,
    color: bool,
}

///SPOOLing
// struct AppState {
//     spooling: Arc<SPOOLing>
// }

// /NoSPOOLing
struct AppState {
    spooling: Arc<NoSPOOLing>
}

async fn submit_job(
    data: web::Data<AppState>,
    req: web::Json<PrintRequest>,
) -> impl Responder {
    let raw_job = rawJob {
        priority: req.priority,
        team_name: req.team_name.to_string(),
        submit_time: Utc::now(),
        file_content: req.file_content.to_string(),
        color: req.color,
    };

    match data.spooling.submit_job(raw_job) {
        Ok(job_id) => {
            HttpResponse::Ok().json(json!({
                "status": "success",
                "message": "打印任务提交成功",
                "data": {
                    "job_id": job_id
                }
            }))
        }
        Err(e) => {
            HttpResponse::ServiceUnavailable().json(json!({
                "status": "error",
                "message": format!("提交失败: {}", e)
            }))
        }
    }
}

/// 获取spooling系统运行状态
async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let status = data.spooling.get_status();
    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": status
    }))
}


/// 返回完成任务的id
async fn get_active_id(data: web::Data<AppState>) -> impl Responder {
    let ids = data.spooling.get_active_job_id();
    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": {
            "active_job_ids": ids
        }
    }))
}


/// 返回总任务和打印完的任务
async fn count_task() -> impl Responder {
    let (all_task,completed_task)=job::stats();
    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": {
            "all_task":all_task,
            "completed_task":completed_task
        }
    }))
}


#[derive(serde::Deserialize)]
struct JobIdRequest {
    id: u64,
}
async fn get_job_info(
    data: web::Data<AppState>,
    req: web::Json<JobIdRequest>,
) -> impl Responder {
    let job_id = req.id;
    // 假设你有 status_map 或类似结构存储所有 Job
    // 这里以 spooling.status_map 为例
    let status_map = data.spooling.status_map.lock().unwrap();
    if let Some(job) = status_map.get(&job_id) {
        HttpResponse::Ok().json(json!({
            "status": "success",
            "data": job
        }))
    } else {
        HttpResponse::NotFound().json(json!({
            "status": "error",
            "message": "Job not found"
        }))
    }
}

// 获取所有job的信息
async fn get_all_info(data: web::Data<AppState>) -> impl Responder {
    let status_map = data.spooling.status_map.lock().unwrap();
    let jobs: Vec<_> = status_map.values().cloned().collect();
    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": jobs
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 创建打印机和 SPOOLing 系统
    let printer = Arc::new(Printer::new());
    // let spooling = Arc::new(SPOOLing::new(10, 10, 10, 10));

    // // 启动 SPOOLing 工作线程
    // spooling.clone().start_workers(printer);

    // let app_state = web::Data::new(AppState {
    //     spooling: spooling.clone(),
    // });

    let nospooling=Arc::new(NoSPOOLing::new(printer));
    let app_state=web::Data::new(AppState{
        spooling:nospooling,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/print", web::post().to(submit_job))
            .route("/status", web::get().to(get_status)) // 改为 GET 路由
            .route("/get_active_id", web::get().to(get_active_id))
            .route("/count_task", web::get().to(count_task))
            .route("/get_job_info", web::post().to(get_job_info))
            .route("/get_all_info", web::get().to(get_all_info))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}