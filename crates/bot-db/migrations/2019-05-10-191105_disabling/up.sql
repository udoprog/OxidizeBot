ALTER TABLE promotions ADD COLUMN disabled BOOLEAN DEFAULT false;
ALTER TABLE promotions ADD COLUMN "group" TEXT;
CREATE INDEX idx_promotions_group ON promotions("group");

ALTER TABLE aliases ADD COLUMN disabled BOOLEAN DEFAULT false;
ALTER TABLE aliases ADD COLUMN "group" TEXT;
CREATE INDEX idx_aliases_group ON aliases("group");

ALTER TABLE commands ADD COLUMN disabled BOOLEAN DEFAULT false;
ALTER TABLE commands ADD COLUMN "group" TEXT;
CREATE INDEX idx_commands_group ON commands("group");

DROP TABLE set_values;