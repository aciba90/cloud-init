
/// Remove quotes from quoted value.
pub fn unquote(val: &str) -> &str {
    const QUOTE: char = '"';
    const TICK: char = '\'';

    if val.starts_with(TICK) && val.ends_with(TICK) {
        return val.strip_prefix(TICK).unwrap().strip_suffix(TICK).unwrap();
    }
 
    // XXX: duplicated code
    if val.starts_with(QUOTE) && val.ends_with(QUOTE) {
        return val.strip_prefix(QUOTE).unwrap().strip_suffix(QUOTE).unwrap();
    }

    val
}

/// parse a yaml single line array value ([1,2,3], not key: [1,2,3]).
/// supported with or without leading and closing brackets
///   ['1'] or [1]
///   '1', '2'
pub fn parse_yaml_array(val: &str) -> Vec<&str> {
    let val = val.strip_prefix('[').unwrap_or_else(|| val);
    let val = val.strip_prefix(']').unwrap_or_else(|| val);
    val.split(',').map(|tok| unquote(tok.trim())).collect()
}


#[cfg(test)]
mod utils {
    use super::*;
    
    #[test]
    fn test_unquote() {
        assert_eq!("a", unquote("a"));
        assert_eq!("a", unquote("'a'"));
        assert_eq!("a", unquote("\"a\""));
    }

    #[test]
    fn test_parse_yaml_array() {
        assert_eq!(vec!["a"], parse_yaml_array("a"));
        assert_eq!(vec!["a", "b"], parse_yaml_array("a,b"));
        assert_eq!(vec!["a", "b"], parse_yaml_array("'a' ,  \"b\""));
    }

}
