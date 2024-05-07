-- Active: 1703754780028@@127.0.0.1@3306@runes
CREATE TABLE `rune_event` (
  `id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `block` BIGINT UNSIGNED NOT NULL,
  `tx_id` VARCHAR(256) NOT NULL,
  `event_type` TinyInt UNSIGNED NOT NULL DEFAULT 0 COMMENT '0:etch/1:mint/2:transfer',
  `rune_id` VARCHAR(64) NOT NULL,
  `address` VARCHAR(256) NOT NULL DEFAULT '',
  `pk_script_hex` VARCHAR(256) NOT NULL DEFAULT '',
  `amount` decimal(40,0) NULL,
  `vout` INT UNSIGNED NOT NULL DEFAULT 0,
  `rune_stone` TEXT NOT NULL,
  `timestamp` BIGINT UNSIGNED NOT NULL DEFAULT 0,
  CONSTRAINT `PRIMARY` PRIMARY KEY (`id`)
);