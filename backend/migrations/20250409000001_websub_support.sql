-- Add hub_url to feeds table for WebSub support
ALTER TABLE feeds ADD COLUMN hub_url TEXT;
