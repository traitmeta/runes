// @generated automatically by Diesel CLI.

diesel::table! {
    etching (id) {
        id -> Unsigned<Bigint>,
        rune_id -> Decimal,
        #[max_length = 256]
        destination -> Varchar,
        #[max_length = 256]
        commit_tx_id -> Varchar,
        commit_tx -> Text,
        #[max_length = 256]
        reveal_tx_id -> Varchar,
        reveal_tx -> Text,
        inscription_output -> Text,
        create_at -> Datetime,
        update_at -> Datetime,
    }
}

diesel::table! {
    rune_entry (id) {
        id -> Unsigned<Bigint>,
        block -> Bigint,
        burned -> Decimal,
        divisibility -> Integer,
        #[max_length = 256]
        etching -> Varchar,
        #[max_length = 64]
        spaced_rune -> Varchar,
        premine -> Decimal,
        mints -> Decimal,
        number -> Bigint,
        timestamp -> Bigint,
        #[max_length = 64]
        rune_id -> Varchar,
        turbo -> Bool,
        #[max_length = 8]
        symbol -> Varchar,
        amount -> Nullable<Decimal>,
        cap -> Nullable<Decimal>,
        height_start -> Nullable<Unsigned<Bigint>>,
        height_end -> Nullable<Unsigned<Bigint>>,
        offset_start -> Nullable<Unsigned<Bigint>>,
        offset_end -> Nullable<Unsigned<Bigint>>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    etching,
    rune_entry,
);
