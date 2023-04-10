ALTER TABLE commands ADD COLUMN count INTEGER DEFAULT 0;
INSERT INTO commands (channel, name, count, text) SELECT channel, name, count, text FROM counters;
DROP TABLE counters;