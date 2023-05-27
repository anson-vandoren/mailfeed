// @generated automatically by Diesel CLI.

diesel::table! {
    feed_items (id) {
        id -> Nullable<Integer>,
        feed_id -> Integer,
        title -> Text,
        link -> Text,
        pub_date -> Integer,
        description -> Nullable<Text>,
        author -> Nullable<Text>,
        categories -> Nullable<Text>,
    }
}

diesel::table! {
    feeds (id) {
        id -> Nullable<Integer>,
        url -> Text,
        feed_type -> Text,
        title -> Text,
        last_checked -> Integer,
        last_updated -> Integer,
        error_time -> Integer,
        error_message -> Nullable<Text>,
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
        id -> Nullable<Integer>,
        user_id -> Integer,
        friendly_name -> Text,
        frequency -> Text,
        last_sent_time -> Integer,
        max_items -> Integer,
        is_active -> Bool,
        feed_id -> Nullable<Integer>,
    }
}

diesel::table! {
    users (id) {
        id -> Nullable<Integer>,
        login_email -> Text,
        send_email -> Text,
        password -> Text,
        created_at -> Integer,
        is_active -> Bool,
        daily_send_time -> Text,
        roles -> Text,
    }
}

diesel::joinable!(feed_items -> feeds (feed_id));
diesel::joinable!(subscriptions -> feeds (feed_id));
diesel::joinable!(subscriptions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    feed_items,
    feeds,
    settings,
    subscriptions,
    users,
);
