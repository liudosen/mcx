ALTER TABLE admin_users
    ADD COLUMN permission_codes LONGTEXT NULL AFTER role;

UPDATE admin_users
SET permission_codes = '[]'
WHERE permission_codes IS NULL OR permission_codes = '';

ALTER TABLE admin_users
    MODIFY permission_codes LONGTEXT NOT NULL;
