# Initializing Database

Initialize postgres database

```
CREATE EXTENSION IF NOT EXISTS "uuid-ossp"; 

CREATE TABLE users (
client_id UUID DEFAULT uuid_generate_v4(),
username VARCHAR(64) UNIQUE NOT NULL,
password_sha256 VARCHAR(64) NOT NULL
);

CREATE TABLE posts (
poster_id UUID NOT NULL,
post_body VARCHAR(512) NOT NULL,
post_time TIMESTAMP NOT NULL
);
```