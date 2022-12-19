pub type Hash = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    name: String,
    hash: Hash,
}

impl Identifier {
    pub fn new<S: Into<String>>(id: S) -> Self {
        let name = id.into();
        let hash = hash::hash_65599(&name);

        Self { name, hash }
    }

    pub fn hash(&self) -> Hash {
        self.hash
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}


mod hash {
    const HASH_CONST: u32 = 65599;

    pub fn hash_65599(id: &str) -> u32 {
        let mut hash = id.len() as u32;
        let mut coef = HASH_CONST;

        for chr in id.chars() {
            hash = hash.wrapping_add(coef.wrapping_mul(chr as u32));
            coef = coef.wrapping_mul(HASH_CONST);
        }

        hash
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_known_id_hashes() {
        assert_eq!(Identifier::new("maestro_pw.Maestro").hash(), 0x7ede71ea);
        assert_eq!(Identifier::new("GetSoftwareInfo").hash(), 0x7199fa44);
        assert_eq!(Identifier::new("SubscribeToSettingsChanges").hash(), 0x2821adf5);
    }
}
