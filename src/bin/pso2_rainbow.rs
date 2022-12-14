use generic_array::typenum::U16;
use generic_array::GenericArray;
use itertools::Itertools;
use lazy_regex::regex_is_match;
use md5::{Digest, Md5};
use pso2_rainbow::{
    models::NewHashMapping,
    rainbow_table::{build_graphemes, validate_permutation_bounds},
    *,
};
use rayon::prelude::*;
use tokio::runtime::Builder;
use uuid::Uuid;

// '/' is handled by prefixes, so it's not included in the charset here.
// 'qwyz_' is being removed to reduce complexity temporarily.
const CHARSET: &str = "abcdefghijklmnoprstuvx0123456789";

fn hash_string(string: &str) -> GenericArray<u8, U16> {
    let mut hasher = Md5::new();
    hasher.update(string.chars().map(|c| c as u8).collect_vec());
    hasher.finalize()
}

fn main() {
    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

    let connection_pool = get_connection_pool();

    let grapheme_max_length = 3;
    let graphemes = build_graphemes(&CHARSET.chars().collect_vec(), grapheme_max_length, |s| {
        // This regex matches the following sequences:
        // - 2+ consecutive '_'
        // - '\d[a-z]\d' (digit-letter-digit)
        !regex_is_match!(r"(_{2,}|\d[a-z]\d)", s)
    });

    println!("Graphemes: {}", graphemes.len());

    // Register known file prefixes and suffixes. Each prefix or suffix added to this
    // will increase the database size by a significant amount, so don't add anything
    // unnecessary.
    let prefixes = vec![
        String::from(""), // No prefix; this is an important catch-all
        //String::from("benchmark"),
        //String::from("db_"),
        //String::from("ef_effect"),
        //String::from("empty_"),
        //String::from("it_"),
        //String::from("pl_"),
        //String::from("square_"),
        String::from("sy_"),
        //String::from("ui_"),
        //String::from("ui_gacha_"),
        //String::from("actor/"),
        //String::from("actor/effect/effect_"),
        //String::from("atmos/"),
        //String::from("apc/"),
        //String::from("apc/apc_"),
        //String::from("character/"),
        String::from("character/np_common_"),
        String::from("character/np_reboot_"),
        String::from("character/np_reboot_region_"),
        //String::from("character/making/"),
        //String::from("character/making_reboot/"),
        //String::from("character/making_reboot_ex/"),
        //String::from("character/motion"),
        //String::from("character/pl_"),
        //String::from("chronos/"),
        //String::from("debug/"),
        //String::from("diva/"),
        //String::from("division/"),
        //String::from("estate/"),
        //String::from("enemy/"),
        //String::from("guildroom/"),
        //String::from("interface/"),
        //String::from("interface/ui_"),
        //String::from("item/"),
        String::from("item/cache/it_name_"),
        //String::from("item/weapon/"),
        //String::from("lobby_action/"),
        //String::from("mag/"),
        //String::from("movie/"),
        //String::from("npc/"),
        //String::from("object/"),
        //String::from("object/language/"),
        //String::from("player/"),
        //String::from("player/pl_"),
        //String::from("quest/"),
        String::from("quest/text_reboot_"),
        String::from("quest/text_reboot_world1_region"),
        //String::from("rebops/"),
        //String::from("region_mag/"),
        //String::from("region_mag/language/"),
        //String::from("scad/"),
        //String::from("section_fence/"),
        //String::from("set/"),
        //String::from("skit/"),
        //String::from("skit/sk_"),
        //String::from("sound/"),
        //String::from("stage/"),
        //String::from("trial/"),
        //String::from("trial/trial_"),
        //String::from("window/"),
        //String::from("window/labo"),
        //String::from("world_trial/"),
        //String::from("world_trial/world_trial"),
    ];
    let suffixes = vec![
        String::from("common.ice"),
        String::from("reboot.ice"),
        //String::from("_base.ice"),
        String::from("_cache_appendix.ice"),
        //String::from("_common.ice"),
        //String::from("_common_ex.ice"),
        String::from("_common_reboot.ice"),
        String::from("_common_text.ice"),
        //String::from("_data.ice"),
        //String::from("_enl.ice"),
        //String::from("_ex.ice"),
        String::from("_filelist.ice"),
        String::from("_info.ice"),
        //String::from("_making.ice"),
        //String::from("_making_next.ice"),
        //String::from("_rad.ice"),
        //String::from("_reboot.ice"),
        //String::from("_set_s.ice"),
        String::from("_set_syrinx.ice"),
        //String::from("_skit.ice"),
        //String::from("_tex.ice"),
        //String::from("_text.ice"),
        //String::from("_voice.ice"),
        //String::from("_wtr.ice"),
        //String::from(".cpk"),
        //String::from(".crbp"),
        String::from(".ice"),
        //String::from(".usm"),
        //String::from("/ln_area_template_common_reboot.ice"),
        //String::from("/ln_area_template_common.ice"),
    ];

    println!("Prefixes: {}", prefixes.len());
    println!("Suffixes: {}", suffixes.len());

    // Build input strings
    let (permuted_min_len, permuted_max_len) =
        validate_permutation_bounds(0, 6, grapheme_max_length);
    let graphemes_per_str = permuted_max_len - permuted_min_len;

    println!("Graphemes per string: {}", graphemes_per_str);
    println!(
        "Strings to generate: {}",
        prefixes.len() * suffixes.len() * graphemes.len().pow(graphemes_per_str as u32)
    );

    let plaintext_chunk_size = 100000;
    let plaintext_chunks = prefixes
        .into_iter()
        .cartesian_product(
            graphemes
                .clone()
                .into_iter()
                .permutations(graphemes_per_str)
                .cartesian_product(suffixes.into_iter()),
        )
        .chunks(plaintext_chunk_size);

    let hashes = &mut Vec::with_capacity(plaintext_chunk_size);
    for chunk in &plaintext_chunks {
        // Hash the input strings in parallel
        chunk
            .collect_vec()
            .par_iter()
            .map(|(prefix, (g, suffix))| {
                [vec![prefix.clone()], g.clone(), vec![suffix.clone()]]
                    .into_iter()
                    .concat()
                    .into_iter()
                    .reduce(|accum, item| accum + &item)
                    .expect("iterator should not be empty")
            })
            .map(|s| (hash_string(&s), s))
            .collect_into_vec(hashes);

        // Batch-insert the hashes into the database
        let handle_count = 10;
        let handles = &mut Vec::with_capacity(handle_count);
        for batch in hashes.chunks(plaintext_chunk_size / handle_count) {
            let batch = batch.to_owned();
            let mut connection = connection_pool
                .get()
                .expect("expected a connection from the connection pool");
            handles.push(runtime.spawn(async move {
                create_hash_mappings(
                    &mut connection,
                    &batch
                        .into_iter()
                        .map(|(hash, filename)| NewHashMapping {
                            md5: Uuid::from_bytes(hash.into()),
                            filename,
                        })
                        .collect_vec(),
                );
            }));
        }

        for handle in handles {
            runtime.block_on(handle).unwrap();
        }
    }
}
