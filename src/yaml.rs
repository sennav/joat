use std::collections::{ HashMap, BTreeMap };
use yaml_rust::{ Yaml, YamlLoader };
use crate::template;
use std::path::Path;
use std::fs;

fn merge_hash(overrider: &BTreeMap<Yaml, Yaml>, overriden: &BTreeMap<Yaml, Yaml>) -> Yaml {
    let sub_key = Yaml::String(String::from("subcommands"));
    let r_subcmd = overrider[&sub_key].clone().into_vec()
        .expect("Subcommands should be an array");
    let mut cmds = Vec::new();
    let mut r_map = BTreeMap::new();
    for value in r_subcmd {
        let scmd_hash = value.clone().into_hash().expect(&format!("Invalid subcommand {:?}", value));
        let scmd_name = scmd_hash.keys().nth(0)
            .expect(&format!("Invalid subcommand name {:?}", scmd_hash));
        r_map.insert(scmd_name.to_owned(), true);
        cmds.push(value);
    }
    let n_subcmd = overriden[&sub_key].clone().into_vec()
        .expect("Subcommands should be an array");
    for v in n_subcmd {
        let scmd_hash = v.as_hash().expect(&format!("Invalid subcommand {:?}", v));
        let scmd_name = scmd_hash.keys().nth(0)
            .expect(&format!("Invalid subcommand name {:?}", scmd_hash));

        if !r_map.contains_key(scmd_name) {
            cmds.push(v);
        }
    }

    let mut result = BTreeMap::new();
    for (k, v) in overrider.iter() {
        result.insert(k.clone(), v.clone());
    }
    for (k, v) in overrider.iter() {
        if !result.contains_key(v) {
            result.insert(k.clone(), v.clone());
        }
    }
    result.insert(sub_key, Yaml::Array(cmds));
    return Yaml::Hash(result);
}

fn combine_yaml(overrider: &Yaml, overriden: &Yaml) -> Yaml {
    match (overrider, overriden) {
        (Yaml::Hash(r), Yaml::Hash(n)) => merge_hash(&r, &n),
        _ => return overrider.clone(),
    }
}

fn get_config_from(cmd_name: &String, base_dir: &String) -> Option<Yaml> {
    let local_path = String::from(format!("{}{}.yml", base_dir, cmd_name));
    if Path::new(&local_path).exists() {
        let local_config = fs::read_to_string(local_path)
            .expect("Could not find configuration file");
        let local_yaml = &YamlLoader::load_from_str(&local_config)
            .expect("failed to load YAML file")[0];
        return Some(local_yaml.clone());
    }
    return None;
}

fn get_home_config(cmd_name: &String) -> Option<Yaml> {
    let home_dir_path = match dirs::home_dir() {
        Some(h) => h,
        _ => return None,
    };
    let home_dir_str = home_dir_path.into_os_string().into_string().unwrap();
    let home_path_str = String::from(format!("{}/.{}.joat/", home_dir_str, cmd_name));
    return get_config_from(&cmd_name, &home_path_str);
}

fn get_local_config(cmd_name: &String) -> Option<Yaml> {
    let prod_path = String::from(format!(".{}.joat/", cmd_name));
    match get_config_from(&cmd_name, &prod_path) {
        Some(c) => Some(c),
        None => get_config_from(&cmd_name, &String::from("")),
    }
}

pub fn get_yaml_config(cmd_name :String) -> Yaml {
    let home_config = get_home_config(&cmd_name);
    let local_config = get_local_config(&cmd_name);
    match (home_config, local_config) {
        (Some(h), Some(l)) => combine_yaml(&h, &l),
        (Some(h), None) => h,
        (None, Some(l)) => l,
        (None, None) => {
            println!("Could not find config file");
            ::std::process::exit(1);
        }
    }
}

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