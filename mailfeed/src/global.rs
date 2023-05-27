use diesel::SqliteConnection;
use once_cell::sync::OnceCell;

use crate::models::settings::{NewSetting, Setting};

pub static JWT_SECRET: OnceCell<String> = OnceCell::new();

pub fn init_jwt_secret(conn: &mut SqliteConnection) {
    let secret = get_jwt_secret(conn).unwrap();
    JWT_SECRET.set(secret).expect("Failed to set JWT secret");
}

fn get_jwt_secret(conn: &mut SqliteConnection) -> Option<String> {
    use crate::schema::settings::dsl::*;
    use diesel::prelude::*;

    let result = settings
        .filter(key.eq("jwt_secret"))
        .select(value)
        .first::<String>(conn);

    if let Ok(secret) = result {
        return Some(secret);
    }

    let jwt_setting = NewSetting {
        user_id: None,
        key: "jwt_secret".to_string(),
        value: generate_jwt_secret(),
    };

    match Setting::add(conn, &jwt_setting) {
        Ok(setting) => Some(setting.value),
        Err(_) => None,
    }
}

fn generate_jwt_secret() -> String {
    use rand::distributions::Alphanumeric;
    use rand::{rngs::OsRng, Rng};

    let rng = OsRng;
    rng.sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}
