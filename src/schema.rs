// @generated automatically by Diesel CLI.

diesel::table! {
    email_configs (id) {
        id -> Nullable<Integer>,
        user_id -> Integer,
        smtp_host -> Text,
        smtp_port -> Integer,
        smtp_username -> Text,
        smtp_password -> Text,
        smtp_use_tls -> Bool,
        from_email -> Text,
        from_name -> Nullable<Text>,
        is_active -> Bool,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    feed_items (id) {
        id -> Integer,
        feed_id -> Integer,
        title -> Text,
        link -> Text,
        pub_date -> Integer,
        description -> Nullable<Text>,
        author -> Nullable<Text>,
    }
}

diesel::table! {
    feeds (id) {
        id -> Integer,
        url -> Text,
        feed_type -> Integer,
        title -> Text,
        last_checked -> Integer,
        last_updated -> Integer,
        error_time -> Integer,
        error_message -> Nullable<Text>,
    }
}

diesel::table! {
    sessions (id) {
        id -> Integer,
        session_id -> Text,
        user_id -> Integer,
        expires_at -> Integer,
        created_at -> Integer,
        last_accessed -> Integer,
    }
}

diesel::table! {
    settings (id) {
        id -> Nullable<Integer>,
        user_id -> Nullable<Integer>,
        key -> Text,
        value -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    subscriptions (id) {
        id -> Integer,
        user_id -> Integer,
        friendly_name -> Text,
        frequency -> Integer,
        last_sent_time -> Integer,
        max_items -> Integer,
        is_active -> Bool,
        feed_id -> Integer,
        delivery_method -> Integer,
    }
}

diesel::table! {
    telegram_config (id) {
        id -> Nullable<Integer>,
        bot_token -> Text,
        webhook_url -> Nullable<Text>,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        login_email -> Text,
        send_email -> Text,
        password -> Text,
        created_at -> Integer,
        is_active -> Bool,
        daily_send_time -> Text,
        role -> Text,
        refresh_token -> Nullable<Text>,
        telegram_chat_id -> Nullable<Text>,
        telegram_username -> Nullable<Text>,
    }
}

diesel::joinable!(email_configs -> users (user_id));
diesel::joinable!(feed_items -> feeds (feed_id));
diesel::joinable!(sessions -> users (user_id));
diesel::joinable!(subscriptions -> feeds (feed_id));
diesel::joinable!(subscriptions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    email_configs,
    feed_items,
    feeds,
    sessions,
    settings,
    subscriptions,
    telegram_config,
    users,
);
