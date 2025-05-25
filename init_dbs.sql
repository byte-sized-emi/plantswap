-- CREATE DATABASE IF NOT EXISTS keycloak;
SELECT 'CREATE DATABASE keycloak'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'keycloak')\gexec

-- CREATE DATABASE IF NOT EXISTS plantswap;
SELECT 'CREATE DATABASE plantswap'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'plantswap')\gexec
