// @generated automatically by Diesel CLI.

diesel::table! {
    hash_mapping (md5) {
        md5 -> Uuid,
        filename -> Text,
    }
}
