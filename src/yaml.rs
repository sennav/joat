use std::collections::HashMap;
use yaml_rust::Yaml;
use crate::template;

pub fn get_string_from_yaml(yaml: &Yaml) -> String {
    match yaml.clone().into_string() {
        Some(s) => s,
        None => {
            println!("Failed to convert {:?} into string, exiting.", yaml);
            ::std::process::exit(1);
        },
    }
}

pub fn get_hash_from_yaml(yaml: &Yaml, context: &HashMap<String, HashMap<String, String>>) -> HashMap<String, String> {
    let yaml_btree = match yaml.clone().into_hash() {
        Some(t) => t,
        None => {
            if yaml.is_badvalue() {
                return HashMap::new();
            }
            println!("Failed to convert to hash map, exiting.");
            ::std::process::exit(1);
        }
    };
    let mut yaml_hash = HashMap::new();
    for (key, value) in yaml_btree.iter() {
        let str_key = get_string_from_yaml(key);
        let value_str = get_string_from_yaml(value);
        let parsed_value = match template::get_compiled_template_str_with_context(&value_str, &context) {
            Ok(t) => t,
            Err(_e) => continue,
        };
        yaml_hash.insert(str_key, parsed_value);
    };
    return yaml_hash;
}

pub fn get_subcommand_from_yaml(cmd_name: &str, yaml: &Yaml) -> Yaml {
    let subcommands = &yaml["subcommands"];
    let subcommands_vec = match subcommands.clone().into_vec() {
        Some(t) => t,
        None => {
            println!("Failed to retrieve subcommands, exiting.");
            ::std::process::exit(1);
        }
    };
    let cmd_name_yaml = Yaml::from_str(cmd_name);
    let scmd_option = subcommands_vec.iter().find(|&s| {
        match s.clone().into_hash() {
            Some(sl) => sl.contains_key(&cmd_name_yaml),
            None => false
        }
    });
    let scmd_hash = match scmd_option {
        Some(s) => s,
        None => {
            println!("Failed to retrieve subcommands hash, exiting.");
            ::std::process::exit(1);   
        }
    };
    return scmd_hash[cmd_name].clone();
}
