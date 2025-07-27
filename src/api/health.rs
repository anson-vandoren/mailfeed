use actix_web::{get, web, HttpResponse, Responder};
use crate::RqDbPool;
use serde_json::json;

/// Health check endpoint for load balancers
#[get("")]
pub async fn health_check(pool: RqDbPool) -> impl Responder {
    // Simple health check - try to get a database connection
    match pool.get() {
        Ok(_) => HttpResponse::Ok().json(json!({
            "status": "healthy",
            "database": "connected"
        })),
        Err(_) => HttpResponse::ServiceUnavailable().json(json!({
            "status": "unhealthy",
            "database": "disconnected"
        }))
    }
}

/// Readiness check - more detailed than health check
#[get("/ready")]
pub async fn readiness_check(pool: RqDbPool) -> impl Responder {
    match pool.get() {
        Ok(_) => HttpResponse::Ok().json(json!({
            "status": "ready",
            "database": "connected"
        })),
        Err(_) => HttpResponse::ServiceUnavailable().json(json!({
            "status": "not_ready",
            "database": "disconnected"
        }))
    }
}

/// Liveness check - simple check to see if the app is alive
#[get("/live")]
pub async fn liveness_check() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "alive",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Basic metrics endpoint
#[get("/metrics")]
pub async fn metrics_endpoint(pool: RqDbPool) -> impl Responder {
    let db_status = if pool.get().is_ok() { 1 } else { 0 };
    
    let prometheus_metrics = format!(
        "# HELP mailfeed_database_status Database health status (1=healthy, 0=unhealthy)\n\
         # TYPE mailfeed_database_status gauge\n\
         mailfeed_database_status {}\n",
        db_status
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