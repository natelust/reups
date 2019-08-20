/// A base struct to use with various DB implementations.
/// It is not required to use this struct as long as a type implement
/// the interface it may be used as a db source, this exists simply
/// as a starting point and a way to share code. As rust does not have
/// true inheritance, this is implemented with a factory macro that
/// spits out structs of a given name containing all the common fields.
/// Anywhere this struct is used, FnvHashMap must be imported.
macro_rules! make_db_source_struct {
    ($name:ident, $storage:ty $(, $field:ident:$type:ty),*) => {
        #[derive(Debug)]
        pub struct $name {
            pub(crate) location: PathBuf,
            pub(crate) tag_to_product_info: FnvHashMap<String, FnvHashMap<String, $storage>>,
            pub(crate) product_to_version_info: FnvHashMap<String, FnvHashMap<String, $storage>>,
            pub(crate) product_to_tags: FnvHashMap<String, Vec<String>>,
            pub(crate) product_to_ident: Option<FnvHashMap<String, Vec<String>>>,
            pub(crate) product_ident_version: Option<FnvHashMap<String, FnvHashMap<String, String>>>,
            $(pub(crate) $field:$type)*
        }
    };
}

/// Base implementations for common methods in posix and json
macro_rules! make_db_source_default_methods {
    () => {
    fn get_location(&self) -> &super::PathBuf {
        &self.location
    }

    fn get_products(&self) -> Vec<&str> {
        self.product_to_version_info.keys().map(|a| a.as_str()).collect()
    }

    fn lookup_flavor_version(&self, product: &str, version: &str) -> Option<&str> {
        Some(self.product_to_version_info
            .get(product)?
            .get(version)?
            .get("FLAVOR")?.as_ref())
    }

    fn get_tags(&self, product: &str) -> Option<Vec<&str>> {
        Some(
            self.product_to_tags
                .get(product)?
                .iter()
                .map(|a| a.as_str())
                .collect(),
        )
    }

    fn get_versions(&self, product: &str) -> Option<Vec<&str>> {
        Some(
            self.product_to_version_info
                .get(product)?
                .keys()
                .map(|a| a.as_str())
                .collect(),
        )
    }

    fn get_identities(&self, product: &str) -> Option<Vec<&str>> {
        Some(
            self.product_to_ident
                .as_ref()?
                .get(product)?
                .iter()
                .map(|a| a.as_str())
                .collect(),
        )
    }

    fn lookup_version_tag(&self, product: &str, tag: &str) -> Option<&str> {
        Some(
            self.tag_to_product_info
                .get(tag)?
                .get(product)?
                .get("VERSION")?,
        )
    }

    fn lookup_version_ident(&self, product: &str, ident: &str) -> Option<&str> {
        Some(
            self.product_ident_version
                .as_ref()?
                .get(product)?
                .get(ident)?
                .as_str(),
        )
    }

    fn lookup_location_version(&self, product: &str, version: &str) -> Option<&PathBuf> {
        if self
            .product_to_version_info
            .get(product)?
            .get(version)
            .is_some()
        {
            Some(&self.location)
        } else {
            None
        }
    }

    fn has_identity(&self, product: &str, ident: &str) -> bool {
        self.product_ident_version.is_some()
            && self
                .product_ident_version
                .as_ref()
                .unwrap()
                .get(product)
                .is_some()
            && self
                .product_ident_version
                .as_ref()
                .unwrap()
                .get(product)
                .unwrap()
                .contains_key(ident)
    }

    fn has_product(&self, product: &str) -> bool {
        self.product_to_version_info.contains_key(product)
    }

    fn identities_populated(&self) -> bool {
        self.product_ident_version.is_some()
    }
    };
}
