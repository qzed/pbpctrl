pub type Hash = u32;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id {
    name: String,
}

impl Id {
    pub fn new(id: impl Into<String>) -> Self {
        Self { name: id.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn hash(&self) -> Hash {
        hash::hash_65599(&self.name)
    }

    pub fn as_ref(&self) -> IdRef<'_> {
        IdRef { name: &self.name }
    }
}

impl<S> From<S> for Id
where
    S: Into<String>
{
    fn from(name: S) -> Self {
        Id::new(name)
    }
}

impl<'a> From<IdRef<'a>> for Id {
    fn from(id: IdRef<'a>) -> Self {
        Id::new(id.name())
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdRef<'a> {
    name: &'a str,
}

impl<'a> IdRef<'a> {
    pub fn new(name: &'a str) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn hash(&self) -> Hash {
        hash::hash_65599(self.name)
    }
}

impl<'a> From<&'a str> for IdRef<'a> {
    fn from(name: &'a str) -> Self {
        IdRef::new(name)
    }
}

impl<'a> From<&'a String> for IdRef<'a> {
    fn from(name: &'a String) -> Self {
        IdRef::new(name)
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    path: String,
    split: usize,
}

impl Path {
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        let split = path.rfind('/').unwrap_or(0);

        Path { path, split }
    }

    pub fn service(&self) -> IdRef<'_> {
        IdRef::new(&self.path[..self.split])
    }

    pub fn method(&self) -> IdRef<'_> {
        if self.split < self.path.len() {
            IdRef::new(&self.path[self.split+1..])
        }  else {
            IdRef::new(&self.path[0..0])
        }
    }

    pub fn as_ref(&self) -> PathRef<'_> {
        PathRef { path: &self.path, split: self.split }
    }
}

impl From<&str> for Path {
    fn from(name: &str) -> Self {
        Path::new(name)
    }
}

impl From<String> for Path {
    fn from(name: String) -> Self {
        Path::new(name)
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathRef<'a> {
    path: &'a str,
    split: usize,
}

impl<'a> PathRef<'a> {
    pub fn new(path: &'a str) -> Self {
        let split = path.rfind('/').unwrap_or(0);

        PathRef { path, split }
    }

    pub fn service(&self) -> IdRef<'a> {
        IdRef::new(&self.path[..self.split])
    }

    pub fn method(&self) -> IdRef<'a> {
        if self.split < self.path.len() {
            IdRef::new(&self.path[self.split+1..])
        }  else {
            IdRef::new(&self.path[0..0])
        }
    }
}

impl<'a> From<&'a str> for PathRef<'a> {
    fn from(name: &'a str) -> Self {
        PathRef::new(name)
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
        assert_eq!(IdRef::new("maestro_pw.Maestro").hash(), 0x7ede71ea);
        assert_eq!(IdRef::new("GetSoftwareInfo").hash(), 0x7199fa44);
        assert_eq!(IdRef::new("SubscribeToSettingsChanges").hash(), 0x2821adf5);
    }

    #[test]
    fn test_path() {
        let pref = PathRef::new("maestro_pw.Maestro/GetSoftwareInfo");
        assert_eq!(pref.service().name(), "maestro_pw.Maestro");
        assert_eq!(pref.service().hash(), 0x7ede71ea);
        assert_eq!(pref.method().name(), "GetSoftwareInfo");
        assert_eq!(pref.method().hash(), 0x7199fa44);

        let pref = PathRef::new("maestro_pw.Maestro/SubscribeToSettingsChanges");
        assert_eq!(pref.service().name(), "maestro_pw.Maestro");
        assert_eq!(pref.service().hash(), 0x7ede71ea);
        assert_eq!(pref.method().name(), "SubscribeToSettingsChanges");
        assert_eq!(pref.method().hash(), 0x2821adf5);
    }
}
