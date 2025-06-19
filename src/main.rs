mod job;
mod osim;
mod printer;

use actix_web::{App, HttpResponse, HttpServer, Responder, Result, Error, web};
use chrono::Utc;
use osim::NoSPOOLing::NoSPOOLing;
use osim::SPOOLing::{SPOOLing, rawJob};
use printer::Printer;
use serde_json::json;
use std::sync::Arc;

use actix_files::NamedFile;
use actix_web::error::ErrorInternalServerError;
use actix_web::web::Bytes;
use futures_util::stream::once;
use std::fs;
use std::path::Path;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use zip::write::FileOptions; // 只需加这一行
use chrono::{DateTime};

#[derive(serde::Deserialize)]
struct PrintRequest {
    priority: u32,
    team_name: String,
    file_content: String,
    color: bool,
    problem_name:String,
}

//SPOOLing
struct AppState {
    spooling: Arc<SPOOLing>
}

// /NoSPOOLing
// struct AppState {
//     spooling: Arc<NoSPOOLing>,
// }

async fn submit_job(data: web::Data<AppState>, req: web::Json<PrintRequest>) -> impl Responder {
    let raw_job = rawJob {
        priority: req.priority,
        team_name: req.team_name.to_string(),
        submit_time: Utc::now(),
        file_content: req.file_content.to_string(),
        color: req.color,
        problem_name:req.problem_name.to_string(),
    };

    match data.spooling.submit_job(raw_job) {
        Ok(job_id) => HttpResponse::Ok().json(json!({
            "status": "success",
            "message": "打印任务提交成功",
            "data": {
                "job_id": job_id
            }
        })),
        Err(e) => HttpResponse::ServiceUnavailable().json(json!({
            "status": "error",
            "message": format!("提交失败: {}", e)
        })),
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
    let (all_task, completed_task) = job::stats();
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
async fn get_job_info(data: web::Data<AppState>, req: web::Json<JobIdRequest>) -> impl Responder {
    let job_id = req.id;
    let status_map = data.spooling.status_map.lock().unwrap();

    if let Some(job) = status_map.get(&job_id) {
        let fmt = |dt: &DateTime<_>| dt.format("%Y/%m/%d %H:%M:%S").to_string();

        let json = json!({
            "job_id": job.job_id,
            "priority": job.priority,
            "team_name": job.team_name,
            "file_name": job.file_name,
            "problem_name": job.problem_name,
            "submit_time": fmt(&job.submit_time),
            "file_content": job.file_content,
            "color": job.color,
            "status": job.status,
            "start_print_time": job.start_print_time.as_ref().map(fmt),
            "end_print_time": job.end_print_time.as_ref().map(fmt),
        });

        HttpResponse::Ok().json(json!({
            "status": "success",
            "data": json
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

/// 下载单个文件接口
async fn download_file(
    data: web::Data<AppState>,
    req: web::Json<JobIdRequest>,
) -> actix_web::Result<NamedFile> {
    let job_id = req.id;
    let status_map = data.spooling.status_map.lock().unwrap();
    if let Some(job) = status_map.get(&job_id) {

        let file_path = PathBuf::from(format!("./output/{}.pdf", job.file_name));
        print!("./output/{}", job.clone().file_name);
        Ok(NamedFile::open(file_path)?)
    } else {
        // 返回404
        Err(actix_web::error::ErrorNotFound("Job not found"))
    }
}


async fn download_all_files(data: web::Data<AppState>) -> Result<HttpResponse, actix_web::Error> {
    // 拿所有任务，包括未完成的
    let jobs: Vec<_> = {
        let status_map = data.spooling.status_map.lock().unwrap();
        status_map
            .values()
            .filter(|job| job.status == job::JobStatus::Completed)
            .cloned()
            .collect()
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for job in jobs {
            let file_path = format!("output/{}.pdf", job.file_name);

            match fs::read(&file_path) {
                Ok(content) => {
                    zip.start_file(format!("{}.pdf", job.file_name), options)
                        .map_err(ErrorInternalServerError)?;
                    zip.write_all(&content).map_err(ErrorInternalServerError)?;
                }
                Err(e) => {
                    eprintln!("文件读取失败（可能是未完成任务或文件缺失）：{} -> {}", file_path, e);
                    // 继续打包其他文件，不中断
                }
            }
        }

        zip.finish().map_err(ErrorInternalServerError)?;
    }

    let buffer = cursor.into_inner();

    let stream = once(async move { Ok::<Bytes, actix_web::Error>(Bytes::from(buffer)) });

    Ok(HttpResponse::Ok()
        .content_type("application/zip")
        .append_header(("Content-Disposition", "attachment; filename=\"all_files.zip\""))
        .streaming(stream))
}





#[actix_web::main]
async fn main() -> std::io::Result<()> {

        // 检查 fonts 文件夹是否存在
    if !Path::new("fonts").exists() {
        eprintln!("错误：fonts 文件夹不存在，请先准备字体文件！");
        std::process::exit(1);
    }

    // 检查 output 文件夹是否存在，不存在则自动创建
    if !Path::new("output").exists() {
        fs::create_dir("output")?;
        println!("output 文件夹不存在，已自动创建。");
    }


    // 创建打印机和 SPOOLing 系统
    let printer = Arc::new(Printer::new());
    let spooling = Arc::new(SPOOLing::new(10, 10, 10, 10));

    // 启动 SPOOLing 工作线程
    spooling.clone().start_workers(printer);

    let app_state = web::Data::new(AppState {
        spooling: spooling.clone(),
    });

    // let nospooling = Arc::new(NoSPOOLing::new(printer));
    // let app_state = web::Data::new(AppState {
    //     spooling: nospooling,
    // });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/print", web::post().to(submit_job))
            .route("/status", web::get().to(get_status)) // 改为 GET 路由
            .route("/get_active_id", web::get().to(get_active_id))
            .route("/count_task", web::get().to(count_task))
            .route("/get_job_info", web::post().to(get_job_info))
            .route("/get_all_info", web::get().to(get_all_info))
            .route("/download_file", web::post().to(download_file))
            .route("/download_all", web::get().to(download_all_files))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
