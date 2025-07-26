use actix_web::{get, web, HttpResponse, Responder};
use crate::{observability::{HealthStatus, Metrics}, RqDbPool};
use serde_json::json;

/// Health check endpoint for load balancers
#[get("")]
pub async fn health_check(
    pool: RqDbPool,
    metrics: web::Data<Metrics>,
) -> impl Responder {
    let health = HealthStatus::check(&metrics, &pool).await;
    
    if health.status == "healthy" {
        HttpResponse::Ok().json(health)
    } else {
        HttpResponse::ServiceUnavailable().json(health)
    }
}

/// Readiness check - more detailed than health check
#[get("/ready")]
pub async fn readiness_check(
    pool: RqDbPool,
    metrics: web::Data<Metrics>,
) -> impl Responder {
    let health = HealthStatus::check(&metrics, &pool).await;
    
    // Additional readiness checks could go here
    // For example, checking if background tasks are running
    
    if health.status == "healthy" {
        HttpResponse::Ok().json(json!({
            "status": "ready",
            "health": health,
            "background_tasks": {
                "feed_monitor": "running",
                "email_sender": "running"
            }
        }))
    } else {
        HttpResponse::ServiceUnavailable().json(json!({
            "status": "not_ready",
            "health": health,
            "background_tasks": {
                "feed_monitor": "unknown",
                "email_sender": "unknown"
            }
        }))
    }
}

/// Liveness check - simple check to see if the app is alive
#[get("/live")]
pub async fn liveness_check(metrics: web::Data<Metrics>) -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "alive",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": metrics.uptime_seconds()
    }))
}

/// Metrics endpoint for monitoring systems
#[get("/metrics")]
pub async fn metrics_endpoint(
    pool: RqDbPool,
    metrics: web::Data<Metrics>,
) -> impl Responder {
    let health = HealthStatus::check(&metrics, &pool).await;
    
    // Basic metrics in Prometheus-like format
    let prometheus_metrics = format!(
        "# HELP mailfeed_uptime_seconds Application uptime in seconds\n\
         # TYPE mailfeed_uptime_seconds counter\n\
         mailfeed_uptime_seconds {}\n\
         \n\
         # HELP mailfeed_health_status Application health status (1=healthy, 0=unhealthy)\n\
         # TYPE mailfeed_health_status gauge\n\
         mailfeed_health_status {}\n\
         \n\
         # HELP mailfeed_database_status Database health status (1=healthy, 0=unhealthy)\n\
         # TYPE mailfeed_database_status gauge\n\
         mailfeed_database_status {}\n\
         \n\
         # HELP mailfeed_storage_status Storage health status (1=healthy, 0=unhealthy)\n\
         # TYPE mailfeed_storage_status gauge\n\
         mailfeed_storage_status {}\n",
        health.uptime_seconds,
        if health.status == "healthy" { 1 } else { 0 },
        if health.checks.database == "healthy" { 1 } else { 0 },
        if health.checks.storage == "healthy" { 1 } else { 0 }
    );
    
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(prometheus_metrics)
}

pub fn routes() -> actix_web::Scope {
    web::scope("/health")
        .service(health_check)
        .service(readiness_check)
        .service(liveness_check)
        .service(metrics_endpoint)
}