use crate::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Identifiable)]
#[diesel(table_name = settings)]
pub struct Setting {
    pub id: Option<i32>,
    pub user_id: Option<i32>,
    pub key: String,
    pub value: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = settings)]
pub struct NewSetting {
    pub user_id: Option<i32>,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = settings)]
pub struct UpdateSetting {
    pub value: Option<String>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Setting '{key:?}' already exists for user with id={user_id:?}")]
    SettingExists { key: String, user_id: Option<i32> },
    #[error("Setting '{key:?}' not found for user with id={user_id:?}")]
    SettingNotFound { key: String, user_id: Option<i32> },
    #[error("Database error")]
    DatabaseError,
}

impl Setting {
    pub fn add(conn: &mut SqliteConnection, setting: &NewSetting) -> Result<Setting, Error> {
        use crate::schema::settings::dsl::*;

        // can't add if this key name for this user_id already exists, or
        // for this key name if user_id is None and key name already exists
        // with no user_id
        let setting_exists = match setting.user_id {
            Some(uid) => settings
                .filter(user_id.eq(uid))
                .filter(key.eq(&setting.key))
                .first::<Setting>(conn)
                .optional()
                .expect("Error checking if setting exists"),
            None => settings
                .filter(user_id.is_null())
                .filter(key.eq(&setting.key))
                .first::<Setting>(conn)
                .optional()
                .expect("Error checking if setting exists"),
        };

        if setting_exists.is_some() {
            return Err(Error::SettingExists {
                key: setting.key.clone(),
                user_id: setting.user_id,
            });
        }

        let setting = Setting {
            id: None,
            user_id: setting.user_id,
            key: setting.key.clone(),
            value: setting.value.clone(),
            created_at: chrono::Utc::now().timestamp() as i32,
            updated_at: chrono::Utc::now().timestamp() as i32,
        };

        match diesel::insert_into(settings)
            .values(setting)
            .get_result(conn)
        {
            Ok(setting) => Ok(setting),
            Err(_) => Err(Error::DatabaseError),
        }
    }

    pub fn get(
        conn: &mut SqliteConnection,
        query_key: &str,
        query_user_id: Option<i32>,
    ) -> Result<Setting, Error> {
        use crate::schema::settings::dsl::*;

        let setting = match query_user_id {
            Some(uid) => settings
                .filter(user_id.eq(uid))
                .filter(key.eq(query_key))
                .first::<Setting>(conn),
            None => settings
                .filter(user_id.is_null())
                .filter(key.eq(query_key))
                .first::<Setting>(conn),
        };

        setting.map_err(|_| Error::SettingNotFound {
            key: query_key.to_string(),
            user_id: query_user_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_helpers::get_test_db_connection;

    use super::*;

    #[test]
    fn test_add_system_setting() {
        let mut conn = get_test_db_connection();
        let setting = NewSetting {
            user_id: None,
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };

        let result = Setting::add(&mut conn, &setting).unwrap();
        assert_eq!(result.key, setting.key);
        assert_eq!(result.value, setting.value);
        assert_ne!(Some(result.id), None);
        assert_ne!(Some(result.created_at), None);
        assert_ne!(Some(result.updated_at), None);
        assert_eq!(result.user_id, None);
    }

    #[test]
    fn test_add_user_setting() {
        let mut conn = get_test_db_connection();
        let setting = NewSetting {
            user_id: Some(1),
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };

        let result = Setting::add(&mut conn, &setting).unwrap();
        assert_eq!(result.key, setting.key);
        assert_eq!(result.value, setting.value);
        assert_ne!(Some(result.id), None);
        assert_ne!(Some(result.created_at), None);
        assert_ne!(Some(result.updated_at), None);
        assert_eq!(result.user_id, Some(1));
    }

    #[test]
    fn test_no_dupe_system_setting() {
        let mut conn = get_test_db_connection();
        let setting = NewSetting {
            user_id: None,
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };

        Setting::add(&mut conn, &setting).unwrap();

        let result = Setting::add(&mut conn, &setting);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_dupe_user_setting() {
        let mut conn = get_test_db_connection();
        let setting = NewSetting {
            user_id: Some(1),
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };

        Setting::add(&mut conn, &setting).unwrap();

        let result = Setting::add(&mut conn, &setting);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_system_setting() {
        let mut conn = get_test_db_connection();
        let setting = NewSetting {
            user_id: None,
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };

        Setting::add(&mut conn, &setting).unwrap();

        let result = Setting::get(&mut conn, &setting.key, None).unwrap();
        assert_eq!(result.key, setting.key);
        assert_eq!(result.value, setting.value);
        assert_ne!(Some(result.id), None);
        assert_ne!(Some(result.created_at), None);
        assert_ne!(Some(result.updated_at), None);
        assert_eq!(result.user_id, None);
    }

    #[test]
    fn test_get_user_setting() {
        let mut conn = get_test_db_connection();
        let setting = NewSetting {
            user_id: Some(1),
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };

        Setting::add(&mut conn, &setting).unwrap();

        let result = Setting::get(&mut conn, &setting.key, Some(1)).unwrap();
        assert_eq!(result.key, setting.key);
        assert_eq!(result.value, setting.value);
        assert_ne!(Some(result.id), None);
        assert_ne!(Some(result.created_at), None);
        assert_ne!(Some(result.updated_at), None);
        assert_eq!(result.user_id, Some(1));
    }

    #[test]
    fn test_get_system_setting_not_found() {
        let mut conn = get_test_db_connection();
        let setting = NewSetting {
            user_id: None,
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };

        Setting::add(&mut conn, &setting).unwrap();

        let result = Setting::get(&mut conn, "not_found", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_gets_for_correct_user() {
        let mut conn = get_test_db_connection();
        let setting = NewSetting {
            user_id: Some(1),
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };

        Setting::add(&mut conn, &setting).unwrap();

        // add same key for different user
        let setting = NewSetting {
            user_id: Some(2),
            key: "test_key".to_string(),
            value: "other_value".to_string(),
        };

        Setting::add(&mut conn, &setting).unwrap();

        let result = Setting::get(&mut conn, "test_key", Some(1)).unwrap();
        assert_eq!(result.value, "test_value");
    }
}
