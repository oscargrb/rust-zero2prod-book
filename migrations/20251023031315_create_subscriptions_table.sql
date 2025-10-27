-- Add migration script here
CREATE TABLE subscriptions(
    id uuid not null,
    primary key (id),
    email text not null unique,
    name text not null,
    subscribed_at timestamptz not null
)