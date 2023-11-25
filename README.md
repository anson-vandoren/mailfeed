# Mailfeed

This project is a simple self-hosted service that allows a small number of users
(primarily intended for just one, but can accommodate more) to subscribe to RSS or Atom
feeds and have them delivered to their email inbox.

# Design notes

## Dev setup

- Install Rust and Cargo
- `cargo install diesel_cli --no-default-features --features "sqlite"`
- `sudo apt install libsqlite3-dev`

### Account setup

```sh
cargo run --release -- --create-admin
```

## Data model

### Users

- Users are identified by a unique email address. This is the email address used for login.
- Users may have a sendTo email address, which is the email address to which emails will
  be sent. If this is not set, emails will be sent to the user's email address.
- Users have a password which is hashed and stored in the database.
- Users have a list of subscriptions.
- Users may be active or inactive. Inactive users cannot log in and no emails will be
  sent to them.
- Users may have a "daily send time" configured, which is a time and timezone at which
  daily emails will be sent. If this is not set, daily emails will be sent at midnight
  GMT.
- Users have one or more roles, which may be `admin` or `user`. 
  - An `admin` user can:
    - Create and delete other users (but not themselves).
    - Reset a user's password (their own and others).
    - Perform other maintenance tasks like manual database compaction/cleaning.
    - Set a user's role (their own and others).
    - Set a user's active status (their own and others).
  - A `user` can:
    - Manage their own subscriptions.
    - Change their own password.
    - May change their own sendTo address.
    - Export their own subscriptions to JSON format.
    - Import their own subscriptions from JSON format.
    - Request a password reset email.

### Subscriptions

- Subscriptions reference a particular Feed
- Subscriptions have a name, which is a human-readable name for the Feed. It defaults to
  the title of the Feed, but can be overridden by the user.
- Subscriptions have a have a schedule, which may be `realtime`, `hourly`, or `daily` and
  controls how frequently emails are sent. 
    - `realtime` actually means within a few minutes of the feed being updated, on 
      a TBD polling interval. Probably <5 minutes.
    - `hourly` means that emails will be sent at the top of the hour, if there are new
      items to send.
    - `daily` means that emails will be sent once per day, at a time and timezone chosen
      by the user.
    - For non-`realtime` subscriptions, emails will only be sent if there are new items
      to send, and items accumulated during the interval will not be sent until the next
      interval.
- Subscriptions have a last sent time, which is the time the last email was sent for this
  subscription. This is used to determine whether a new email should be sent.
- Subscriptions have a max items, which is the maximum number of items to include in an
  email. If there are more items slotted for an email than this number, the oldest items
  will only be displayed as links to the content, not as full text.
- Subscriptions may be either active or inactive. Inactive subscriptions will not have
  emails sent for them.
- Subscriptions are associated with one user, and one Feed.

### Feed

- Feeds have a URL, which is the URL of the feed from where the content is pulled.
- Feeds have a type, which may be Atom, RSS, or JSON Feed. This will be determined
  automatically when the feed is added.
- Feeds have a title.
- Feeds have a last checked time for when the service last checked the feed for updates.
- Feeds have a last updated time for the last time the feed was updated.
- Feeds have an error time, which is either null or the first time that an error was
  encountered when trying to update the feed. It is cleared when the feed is updated
  successfully.
- Feeds have an error message, which is either null or the error message that was
  encountered when trying to update the feed. It is cleared when the feed is updated
  successfully. It displays the latest error message, even if the error time is older.
- Feeds are associated with one or more Subscriptions, and zero or more Feed Items.
- Feeds are updated at a TBD polling interval. Probably <5 minutes.

### Feed Items

- Feed Items have a title. If the item does not include one, the description will be used if
  it exists, otherwise the URL will be used if present, otherwise the feed title and date
  will be used.
- Feed Items have a link, which is the URL of the item.
- Feed Items have a publication date. If the item does not include one, the time the item
  was received will be used.
- Feed Items may have a description.
- Feed Items may have an author.
- Feed Items may have one or more categories.

### Notes:

- Need to periodically clean up the database of old Feeds/FeedItems. 
  - Feeds should be deleted if they have no subscriptions, and their FeedItems at the same time. 
  - FeedItems should be deleted if they are older than a certain age. 
    - Static one month?
    - If they are older than the longest subscription frequency for the associated feed?
    - By the size of the table? 
    - Maybe this isn't needed at all, and keeping history could allow for showing 
      some interesting stats.
  - Ideally this is done on an automatic schedule, but we should also probably have a 
    manual way for the admin to do this.

## API:

### Users:

- `GET /api/users` - List all users. Admin only.
- `POST /api/users` - Create a new user. Admin only.
- `GET /api/users/{id}` - Get a user by email. Admin or given user only.
- `PUT /api/users/{id}` - Update a user. Admin or given user only.
- `DELETE /api/users/{id}` - Delete a user. Admin only.

### Authentication:

- `POST /api/auth/login` - Login with email and password, returns a JWT.
- `POST /api/auth/logout` - Logout, invalidates the JWT.
- `POST /api/auth/password-reset` - Request a password reset email.

### Subscriptions:

- `GET /api/users/{id}/subscriptions` - List all subscriptions for a user. User only.
- `POST /api/users/{id}/subscriptions` - Create a new subscription for a user. User only.
- `GET /api/users/{id}/subscriptions/{id}` - Get a subscription by id. User only.
- `PUT /api/users/{id}/subscriptions/{id}` - Update a subscription. User only.
- `DELETE /api/users/{id}/subscriptions/{id}` - Delete a subscription. User only.

### Feeds:

- `GET /api/feeds` - List all feeds. Admin only.
- `POST /api/feeds` - Create a new feed. Admin only.
- `GET /api/feeds/{id}` - Get a feed by id. Admin only.
- `PUT /api/feeds/{id}` - Update a feed. Admin only.
- `DELETE /api/feeds/{id}` - Delete a feed. Admin only.

### Feed Items:

- `GET /api/feeds/{id}/items` - List all feed items for a feed. Admin only.
- `GET /api/feeds/{id}/items/{id}` - Get a feed item by id. Admin only.
