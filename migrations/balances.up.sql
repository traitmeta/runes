-- Active: 1703754780028@@127.0.0.1@3306@runes
CREATE TABLE `rune_balance` (
  `id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `block` BIGINT UNSIGNED NOT NULL,
  `rune_id` VARCHAR(64) NOT NULL,
  `address` VARCHAR(256) NOT NULL DEFAULT '',
  `pk_script_hex` VARCHAR(256) NOT NULL DEFAULT '',
  `out_point` VARCHAR(266) NOT NULL DEFAULT '',
  `amount` decimal(40, 0) NOT NULL DEFAULT 0,
  `spent` BOOLEAN NOT NULL DEFAULT 0,
  CONSTRAINT `PRIMARY` PRIMARY KEY (`id`),
  INDEX `index_rune_id` (`rune_id`)
);