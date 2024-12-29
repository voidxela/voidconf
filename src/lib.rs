/// The `voidconf` library provides a friendly, composable, and extensible framework for configuration management.
/// Currently in alpha development, testing is needed and interfaces may change in the future. But please do report
/// any issues, and pull requests are welcome!
///
/// The core library currently only supports configs from environment variables in a slightly opinionated format;
/// other config sources or unsupported var name schemes can be implemented with a custom [`ConfSource`]. Additional
/// formats will be added over time.
mod err;

pub use err::ConfError;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::str::FromStr;

type Result<T = ()> = core::result::Result<T, ConfError>;

/// Name used by [`Conf::default`]. Env vars should be prefixed `VCFG_`.
pub const DEFAULT_NAME: &str = "vcfg";

/// Generic config value trait. Implement this for any custom types you want to support. This
/// library includes several implementations for commmon types.
pub trait ConfValue:
    Serialize + DeserializeOwned + Clone + std::fmt::Display + FromStr<Err: core::error::Error>
{
}

impl ConfValue for String {}
impl ConfValue for u8 {}
impl ConfValue for u16 {}
impl ConfValue for u32 {}
impl ConfValue for u64 {}
impl ConfValue for i8 {}
impl ConfValue for i16 {}
impl ConfValue for i32 {}
impl ConfValue for i64 {}
impl ConfValue for serde_json::Value {}

/// Source of config values. Can look up from the environment, read from a file, query a server, etc.
pub trait ConfSource {
    /// New [`ConfSource`] should determine where to look for a config based on the given `name`.
    fn new(name: impl Into<String>) -> Self;
    /// Look up a value and return it in serialized string form. Return `None` if not present; default
    /// values are handled in [`Conf::get`].
    fn get(&self, key: impl Into<String>) -> Result<Option<String>>;
}

/// A [`ConfSource`] for resolving prefixed values from environment variables.
pub struct EnvSource {
    /// This should be the value of [`Conf::name`] in uppercase.
    pub prefix: String,
}

impl EnvSource {
    /// Translate a key name into its corresponding env key.
    /// Prepends [`EnvSource::prefix`] and converts to uppercase.
    pub fn env_key(&self, key: impl Into<String>) -> String {
        format!("{}_{}", self.prefix, key.into().to_ascii_uppercase())
    }
}

impl ConfSource for EnvSource {
    /// Create a new [`EnvSource`] with the given name as a [prefix](EnvSource::prefix).
    fn new(name: impl Into<String>) -> Self {
        Self {
            prefix: name.into().to_ascii_uppercase(),
        }
    }

    /// Query the value using the [translated key](EnvSource::env_key) from the environment.
    fn get(&self, key: impl Into<String>) -> Result<Option<String>> {
        let env_key = self.env_key(key);
        match std::env::var(&env_key) {
            Ok(v) => Some(
                v.parse()
                    .map_err(|_| ConfError::val_parse_failed(&env_key, &v)),
            )
            .transpose(),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(e) => Err(ConfError::env_lookup_failed(&env_key, e)),
        }
    }
}

/// Definition of a single conf option.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfEntry<V: ConfValue> {
    /// Conf key name. Must be supported by the target [ConfSource].
    pub name: String,
    /// Conf value type. Any type with a [ConfValue] impl is supported.
    pub val_type: std::marker::PhantomData<V>,
    /// Optional default value. Must deserialize into `V`.
    pub default: Option<String>,
}

impl<V: ConfValue> ConfEntry<V> {
    /// Create a new conf entry with no default. Use [`ConfEntry::with_default`] to add one.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            val_type: std::marker::PhantomData::<V>,
            default: None,
        }
    }

    /// Update this entry to include the given default value.
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }
}

/// This trait allows our [`ConfEntry`]s to all get along in [one big map](Conf::options).
pub trait AnyConfEntry: Send + Sync {
    /// Get a dynamic reference to the struct.
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<V: ConfValue + Send + Sync + 'static> AnyConfEntry for ConfEntry<V> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Top-level conf struct represents a single named config source.
pub struct Conf<S: ConfSource = EnvSource> {
    /// Config name. Source lookups are derived from this.
    pub name: &'static str,
    /// Source for config values. See [`ConfSource`].
    pub source: S,
    /// Map of configured [`ConfEntry`] options.
    pub options: std::collections::BTreeMap<String, Box<dyn AnyConfEntry>>,
}

impl<S: ConfSource> Conf<S> {
    /// Create a new config. Also initializes the [`ConfSource`].
    pub fn new(name: &'static str) -> Self {
        Self {
            source: S::new(name),
            options: std::collections::BTreeMap::new(),
            name,
        }
    }

    /// Add a new [`ConfEntry`]. This is a lower-level function for custom [`ConfValue`] types;
    /// where possible the typed functions such as [`Conf::string`] are preferred.
    pub fn entry<V: ConfValue + Send + Sync + 'static>(mut self, entry: ConfEntry<V>) -> Self {
        self.options.insert(entry.name.clone(), Box::new(entry));
        self
    }

    /// Add a string entry.
    pub fn string(self, name: impl Into<String>, default: Option<&str>) -> Self {
        let entry: ConfEntry<String> = ConfEntry::new(name);
        match default {
            Some(d) => self.entry(entry.with_default(d)),
            None => self.entry(entry),
        }
    }

    /// Add a byte (`u8`) entry.
    pub fn byte(self, name: impl Into<String>, default: Option<u8>) -> Self {
        let entry: ConfEntry<u8> = ConfEntry::new(name);
        match default {
            Some(d) => self.entry(entry.with_default(d.to_string())),
            None => self.entry(entry),
        }
    }

    /// Add an int (`i64`) entry.
    pub fn int(self, name: impl Into<String>, default: Option<i64>) -> Self {
        let entry: ConfEntry<i64> = ConfEntry::new(name);
        match default {
            Some(d) => self.entry(entry.with_default(d.to_string())),
            None => self.entry(entry),
        }
    }

    /// Add a uint (`u64`) entry.
    pub fn uint(self, name: impl Into<String>, default: Option<u64>) -> Self {
        let entry: ConfEntry<u64> = ConfEntry::new(name);
        match default {
            Some(d) => self.entry(entry.with_default(d.to_string())),
            None => self.entry(entry),
        }
    }

    /// Get a value. An error will be thrown if the value cannot parse into the type expected
    /// by the configured entry.
    pub fn get<V: ConfValue + 'static>(&self, key: &str) -> Result<Option<V>> {
        match self.options.get(key) {
            Some(option) => match option.as_any().downcast_ref::<ConfEntry<V>>() {
                Some(entry) => self
                    .source
                    .get(&entry.name)?
                    .or_else(|| entry.default.clone())
                    .map(|v| v.parse().map_err(|_| ConfError::val_parse_failed(key, &v)))
                    .transpose(),
                None => Err(ConfError::val_parse_failed(key, "")),
            },
            None => Err(ConfError::key_not_found(key)),
        }
    }

    /// Get a string value.
    pub fn get_string(&self, key: &str) -> Result<Option<String>> {
        self.get::<String>(key)
    }

    /// Get a byte (`u8`) value.
    pub fn get_byte(&self, key: &str) -> Result<Option<u8>> {
        self.get::<u8>(key)
    }

    /// Get an int (`i64`) value.
    pub fn get_int(&self, key: &str) -> Result<Option<i64>> {
        self.get::<i64>(key)
    }

    /// Get a uint (`u64`) value.
    pub fn get_uint(&self, key: &str) -> Result<Option<u64>> {
        self.get::<u64>(key)
    }

    /// Require a value. Similar to [`Conf::get`] except a `None` return value
    /// is treated as an error.
    pub fn require<V: ConfValue + 'static>(&self, key: &str) -> Result<V> {
        self.get(key)
            .transpose()
            .ok_or_else(|| ConfError::val_not_found(key))?
    }

    /// Require a string value.
    pub fn require_string(&self, key: &str) -> Result<String> {
        self.require::<String>(key)
    }

    /// Require a byte (`u8`) value.
    pub fn require_byte(&self, key: &str) -> Result<u8> {
        self.require::<u8>(key)
    }

    /// Require an int (`i64`) value.
    pub fn require_int(&self, key: &str) -> Result<i64> {
        self.require::<i64>(key)
    }

    /// Require a uint (`u64`) value.
    pub fn require_uint(&self, key: &str) -> Result<u64> {
        self.require::<u64>(key)
    }
}

impl Default for Conf {
    /// Create the default [`Conf`] with [`DEFAULT_NAME`].
    fn default() -> Self {
        Self::new(DEFAULT_NAME)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn clean_env() {
        let vars = vec![
            "test",
            "testy",
            "greeting",
            "name",
            "max_byte",
            "a_number",
            "another_number",
        ];
        vars.iter().for_each(|n| {
            std::env::remove_var(format!("{}_{}", DEFAULT_NAME, n.to_ascii_uppercase()))
        });
    }

    #[test]
    pub fn get_err_key_not_found() {
        clean_env();
        let mut conf = Conf::default();
        assert_eq!(
            conf.get_string("test").unwrap_err(),
            ConfError::KeyNotFound {
                key: "test".to_string()
            }
        );
        conf = conf.string("test", Some("hi"));
        let test = conf.get_string("test").unwrap();
        let testy = conf.get_string("testy").unwrap_err();
        assert_eq!(test, Some("hi".to_string()));
        assert_eq!(
            testy,
            ConfError::KeyNotFound {
                key: "testy".to_string()
            }
        );
    }

    #[test]
    pub fn get_str_default() {
        clean_env();
        let conf = Conf::default().string("name", Some("world"));
        assert_eq!(conf.get_string("name").unwrap(), Some("world".to_string()));
    }

    #[test]
    pub fn get_str_env() {
        clean_env();
        std::env::set_var("VCFG_NAME", "xela");
        let conf = Conf::default().string("name", Some("world"));
        assert_eq!(conf.get_string("name").unwrap(), Some("xela".to_string()));
    }

    #[test]
    pub fn get_str_multi() {
        clean_env();
        let conf = Conf::default()
            .string("greeting", Some("Hello"))
            .string("name", None);
        std::env::set_var("VCFG_NAME", "world");
        let greeting = conf.get_string("greeting").unwrap();
        let name = conf.get_string("name").unwrap();
        assert_eq!(greeting, Some("Hello".to_string()));
        assert_eq!(name, Some("world".to_string()));
    }

    #[test]
    pub fn get_int_multi() {
        clean_env();
        let conf = Conf::default()
            .byte("max_byte", Some(255))
            .int("a_number", Some(-42))
            .uint("another_number", None);
        let a_number = conf.get_int("a_number").unwrap();
        let max_byte = conf.get_byte("max_byte").unwrap();
        let another_number = conf.get_uint("another_number").unwrap();
        assert_eq!(a_number, Some(-42));
        assert_eq!(max_byte, Some(255));
        assert_eq!(another_number, None);
        std::env::set_var("VCFG_MAX_BYTE", "4");
        let new_max_byte = conf.get_byte("max_byte").unwrap();
        assert_eq!(new_max_byte, Some(4));
    }

    #[test]
    pub fn require_str_multi() {
        clean_env();
        let conf = Conf::default()
            .string("greeting", Some("Hello"))
            .string("name", None);
        std::env::set_var("VCFG_NAME", "world");
        let greet = |g: String, n: String| format!("{}, {}!", g, n);
        let conf_greet = || {
            greet(
                conf.require("greeting").unwrap(),
                conf.require("name").unwrap(),
            )
        };
        assert_eq!(conf_greet(), "Hello, world!");
        std::env::set_var("VCFG_NAME", "xela");
        std::env::set_var("VCFG_GREETING", "Hail");
        assert_eq!(conf_greet(), "Hail, xela!");
    }

    #[test]
    pub fn require_int_default() {
        clean_env();
        let conf = Conf::default().uint("count", Some(3));
        let count = conf.require_uint("count").unwrap();
        assert_eq!(count, 3u64);
    }
}
