# Initializing Database

Initialize postgres database

```
CREATE EXTENSION IF NOT EXISTS "uuid-ossp"; 

CREATE TABLE users (
id UUID DEFAULT uuid_generate_v4(),
username VARCHAR UNIQUE NOT NULL,
password_hash VARCHAR NOT NULL,
create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE posts (
id UUID NOT NULL,
body VARCHAR(512) NOT NULL,
time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```
