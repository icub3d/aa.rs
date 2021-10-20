use std::collections::HashMap;

pub type Environment = HashMap<String, String>;

pub trait ApplyEnvironment {
    /// Apply the environment to the given string. Variables in the
    /// string should be surrounded by brackets ({}). That is, if the
    /// environment has a key of ```foo``` and the string contains
    /// ```{foo}```, it will be replaced with the value in the
    /// environment.
    fn apply_environment(&self, s: &str) -> String;
}

impl ApplyEnvironment for [Environment] {
    /// Performs apply_environment but over a list of environments.
    /// Order matters, so the earlier environments will take precedence
    /// and their values will be used if more than one environment has
    /// the same key.
    fn apply_environment(&self, s: &str) -> String {
        let mut s = s.to_string();
        for m in self.iter() {
            for (k, v) in m.iter() {
                s = s.replace(&format!("{{{}}}", k), v).to_string();
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use crate::environment::*;

    macro_rules! hashmap {
        ($( $key: expr => $val: expr ),*) => {{
             let mut map: Environment = ::std::collections::HashMap::new();
             $( map.insert($key.to_string(), $val.to_string()); )*
             map
        }}
    }

    #[test]
    fn precedence_works_as_expected() {
        let envs = vec![
            hashmap!["foo" => "bar"],
            hashmap!["foo" => "baz", "bar" => "buzz"],
        ];
        assert_eq!(
            envs.apply_environment("{foo} {bar}"),
            "bar buzz".to_string()
        );
    }
}
