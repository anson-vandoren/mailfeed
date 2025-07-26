use actix_web::{get, patch, post, web, HttpResponse, Responder};
use std::collections::HashMap;

use super::types::{
    ConfigBulkUpdate, ConfigItem, ConfigResponse, ConfigUpdate, get_config_schemas,
};
use crate::{
    api::users::RqUserId,
    session::SessionClaims,
    models::settings::Setting,
    RqDbPool,
};

#[get("")]
pub async fn get_user_config(
    pool: RqDbPool,
    path: RqUserId,
    claims: SessionClaims,
) -> impl Responder {
    let user_id = match path.user_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID"),
    };

    if claims.sub != user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    // Get user-specific settings
    let user_settings = match Setting::get_all_for_user(&mut conn, Some(user_id)) {
        Ok(settings) => settings,
        Err(_) => Vec::new(),
    };

    // Get system-wide settings (fallback values)
    let system_settings = match Setting::get_all_for_user(&mut conn, None) {
        Ok(settings) => settings,
        Err(_) => Vec::new(),
    };

    // Get configuration schemas
    let schemas = get_config_schemas();
    
    // Build config map with user settings taking precedence over system settings
    let mut config_map = HashMap::new();
    
    // Start with system settings
    for setting in system_settings {
        if let Some(schema) = schemas.iter().find(|s| s.key == setting.key) {
            config_map.insert(
                setting.key.clone(),
                ConfigItem {
                    key: setting.key,
                    value: setting.value,
                    description: Some(schema.description.clone()),
                    config_type: schema.config_type.clone(),
                    category: schema.category.clone(),
                    updated_at: setting.updated_at,
                },
            );
        }
    }
    
    // Override with user settings
    for setting in user_settings {
        if let Some(schema) = schemas.iter().find(|s| s.key == setting.key) {
            config_map.insert(
                setting.key.clone(),
                ConfigItem {
                    key: setting.key,
                    value: setting.value,
                    description: Some(schema.description.clone()),
                    config_type: schema.config_type.clone(),
                    category: schema.category.clone(),
                    updated_at: setting.updated_at,
                },
            );
        }
    }
    
    // Add default values for any missing configs
    for schema in &schemas {
        if !config_map.contains_key(&schema.key) {
            config_map.insert(
                schema.key.clone(),
                ConfigItem {
                    key: schema.key.clone(),
                    value: schema.default_value.clone(),
                    description: Some(schema.description.clone()),
                    config_type: schema.config_type.clone(),
                    category: schema.category.clone(),
                    updated_at: 0,
                },
            );
        }
    }

    let response = ConfigResponse {
        config: config_map,
        schema: schemas,
    };

    HttpResponse::Ok().json(response)
}

#[patch("/{config_key}")]
pub async fn update_user_config(
    pool: RqDbPool,
    user_path: RqUserId,
    config_path: web::Path<String>, // config_key
    update_req: web::Json<ConfigUpdate>,
    claims: SessionClaims,
) -> impl Responder {
    let config_key = config_path.into_inner();
    let user_id_str = &user_path.user_id;
    
    let user_id = match user_id_str.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID"),
    };

    if claims.sub != user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    // Validate that this is a known configuration key
    let schemas = get_config_schemas();
    let schema = match schemas.iter().find(|s| s.key == config_key) {
        Some(schema) => schema,
        None => return HttpResponse::BadRequest().body("Invalid configuration key"),
    };

    // Validate the value according to the schema
    if let Err(error_msg) = validate_config_value(&update_req.value, schema) {
        return HttpResponse::BadRequest().body(error_msg);
    }

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    // Update or create the setting
    match Setting::set(&mut conn, &config_key, Some(user_id), &update_req.value) {
        Ok(setting) => {
            let config_item = ConfigItem {
                key: setting.key,
                value: setting.value,
                description: Some(schema.description.clone()),
                config_type: schema.config_type.clone(),
                category: schema.category.clone(),
                updated_at: setting.updated_at,
            };
            HttpResponse::Ok().json(config_item)
        }
        Err(_) => HttpResponse::InternalServerError().body("Error updating configuration"),
    }
}

#[post("/bulk")]
pub async fn bulk_update_user_config(
    pool: RqDbPool,
    path: RqUserId,
    bulk_update: web::Json<ConfigBulkUpdate>,
    claims: SessionClaims,
) -> impl Responder {
    let user_id = match path.user_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID"),
    };

    if claims.sub != user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let schemas = get_config_schemas();
    let mut updated_configs = HashMap::new();
    let mut errors = Vec::new();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    // Process each update
    for (key, value) in &bulk_update.updates {
        // Validate that this is a known configuration key
        let schema = match schemas.iter().find(|s| s.key == *key) {
            Some(schema) => schema,
            None => {
                errors.push(format!("Invalid configuration key: {}", key));
                continue;
            }
        };

        // Validate the value
        if let Err(error_msg) = validate_config_value(value, schema) {
            errors.push(format!("{}: {}", key, error_msg));
            continue;
        }

        // Update the setting
        match Setting::set(&mut conn, key, Some(user_id), value) {
            Ok(setting) => {
                let config_item = ConfigItem {
                    key: setting.key.clone(),
                    value: setting.value,
                    description: Some(schema.description.clone()),
                    config_type: schema.config_type.clone(),
                    category: schema.category.clone(),
                    updated_at: setting.updated_at,
                };
                updated_configs.insert(setting.key, config_item);
            }
            Err(_) => {
                errors.push(format!("Error updating {}", key));
            }
        }
    }

    if !errors.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "errors": errors,
            "updated": updated_configs
        }));
    }

    HttpResponse::Ok().json(updated_configs)
}

fn validate_config_value(value: &str, schema: &crate::api::config::types::ConfigSchema) -> Result<(), String> {
    use crate::api::config::types::ConfigType;

    let validation = match &schema.validation {
        Some(v) => v,
        None => return Ok(()),
    };

    // Required validation
    if validation.required && value.is_empty() {
        return Err("This field is required".to_string());
    }

    // Type-specific validation
    match schema.config_type {
        ConfigType::Number => {
            let num = value.parse::<i32>()
                .map_err(|_| "Must be a valid number".to_string())?;
            
            if let Some(min) = validation.min {
                if num < min {
                    return Err(format!("Must be at least {}", min));
                }
            }
            
            if let Some(max) = validation.max {
                if num > max {
                    return Err(format!("Must be at most {}", max));
                }
            }
        }
        ConfigType::Boolean => {
            if !matches!(value, "true" | "false") {
                return Err("Must be 'true' or 'false'".to_string());
            }
        }
        ConfigType::String => {
            if let Some(min) = validation.min {
                if value.len() < min as usize {
                    return Err(format!("Must be at least {} characters", min));
                }
            }
            
            if let Some(max) = validation.max {
                if value.len() > max as usize {
                    return Err(format!("Must be at most {} characters", max));
                }
            }
            
            if let Some(pattern) = &validation.pattern {
                let regex = regex::Regex::new(pattern)
                    .map_err(|_| "Invalid pattern configuration".to_string())?;
                if !regex.is_match(value) {
                    return Err("Value does not match required format".to_string());
                }
            }
        }
        ConfigType::Select => {
            if let Some(options) = &schema.options {
                if !options.iter().any(|opt| opt.value == *value) {
                    return Err("Invalid option selected".to_string());
                }
            }
        }
    }

    Ok(())
}