-- Add is_shared column to subscriptions table
ALTER TABLE subscriptions ADD COLUMN is_shared BOOLEAN DEFAULT 0;
