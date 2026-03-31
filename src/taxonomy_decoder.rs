//! The Rosetta Stone: Decoding the biological reality of the simulation.
//!
//! Mandatory Bit-Slicing Contract:
//! Kingdom (0-3), Phylum (4-11), Class (12-23), Order (24-35), Family (36-47), Genus/Species (48-63)

/// Decodes a u64 taxonomic mask into a human-readable Linnaean name.
/// 
/// This function is optimized for the hot loop using bitwise operations
/// and non-allocating match statements where possible.
pub fn decode_taxonomy(mask: u64) -> String {
    let k_val = mask & 0xF;
    let p_val = (mask >> 4) & 0xFF;
    let c_val = (mask >> 12) & 0xFFF;
    let o_val = (mask >> 24) & 0xFFF;
    let f_val = (mask >> 36) & 0xFFF;
    let gs_val = (mask >> 48) & 0xFFFF;

    let kingdom = match k_val {
        0 => "Animalia",
        1 => "Plantae",
        2 => "Fungi",
        x => return format!("Proc-Kingdom-{}", x),
    };

    let phylum = match p_val {
        0 => "Chordata",
        1 => "Arthropoda",
        2 => "Mollusca",
        x => return format!("{} > Proc-Phylum-{}", kingdom, x),
    };

    let class = match c_val {
        0 => "Mammalia",
        1 => "Reptilia",
        2 => "Aves",
        3 => "Dinosauria",
        4 => "Actinopterygii",
        x => return format!("{} > {} > Proc-Class-{}", kingdom, phylum, x),
    };

    let order = match o_val {
        0 => "Carnivora",
        1 => "Primates",
        2 => "Rodentia",
        3 => "Salmoniformes",
        x => return format!("{} > {} > {} > Proc-Order-{}", kingdom, phylum, class, x),
    };

    let family = match f_val {
        0 => "Canidae",
        1 => "Felidae",
        2 => "Hominidae",
        3 => "Salmonidae",
        x => return format!("{} > {} > {} > {} > Proc-Family-{}", kingdom, phylum, class, order, x),
    };

    format!(
        "{} > {} > {} > {} > {} > Sp-{:04X}",
        kingdom, phylum, class, order, family, gs_val
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taxonomic_decoder() {
        // Construct a mask for Homo sapiens:
        // Kingdom: Animalia (0)
        // Phylum: Chordata (0)
        // Class: Mammalia (0)
        // Order: Primates (1)
        // Family: Hominidae (2)
        // Genus/Species: 0x1234
        let human_mask: u64 = (0u64 << 0)   // Kingdom
                           | (0u64 << 4)   // Phylum
                           | (0u64 << 12)  // Class
                           | (1u64 << 24)  // Order
                           | (2u64 << 36)  // Family
                           | (0x1234u64 << 48); // Sp
        
        let result = decode_taxonomy(human_mask);
        assert_eq!(result, "Animalia > Chordata > Mammalia > Primates > Hominidae > Sp-1234");

        // Test procedural fallback for unknown order
        let unknown_order_mask: u64 = (0u64 << 0) | (0u64 << 4) | (0u64 << 12) | (99u64 << 24);
        let result_unknown = decode_taxonomy(unknown_order_mask);
        assert_eq!(result_unknown, "Animalia > Chordata > Mammalia > Proc-Order-99");

        // Test another known species: Canis lupus (Wolf)
        // Animalia (0), Chordata (0), Mammalia (0), Carnivora (0), Canidae (0)
        let wolf_mask: u64 = (0u64 << 24) | (0u64 << 36) | (0xBEEFu64 << 48);
        let result_wolf = decode_taxonomy(wolf_mask);
        assert_eq!(result_wolf, "Animalia > Chordata > Mammalia > Carnivora > Canidae > Sp-BEEF");
    }
}
