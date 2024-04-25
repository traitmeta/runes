-- Active: 1703754780028@@127.0.0.1@3306@runes
CREATE TABLE `rune_entry` ( 
  `id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `block` BIGINT NOT NULL,
  `burned` decimal(40,0) NOT NULL DEFAULT 0,
  `divisibility` INT NOT NULL DEFAULT 0,
  `etching` VARCHAR(256) NOT NULL,
  `spaced_rune` VARCHAR(64) NOT NULL,
  `premine` decimal(40,0) NOT NULL DEFAULT 0,
  `mints` decimal(40,0)  NOT NULL  DEFAULT 0,
  `number` BIGINT NOT NULL DEFAULT 0,
  `timestamp` BIGINT NOT NULL DEFAULT 0,
  `rune_id` VARCHAR(64) NOT NULL,
  `turbo` boolean NOT NULL,
  `symbol` VARCHAR(8) NOT NULL,
  `amount` decimal(40,0) NULL,
  `cap` decimal(40,0)  NULL,
  `height_start` BIGINT UNSIGNED  NULL,
  `height_end` BIGINT UNSIGNED NULL,
  `offset_start` BIGINT UNSIGNED NULL,
  `offset_end` BIGINT  UNSIGNED NULL,
  CONSTRAINT `PRIMARY` PRIMARY KEY (`id`),
  CONSTRAINT `index_spaced_rune` UNIQUE (`spaced_rune`)
);