use crate::template;
use log::debug;
use serde_json::map::Map;
use serde_json::value::Value;
use serde_json::Number;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs};
use yaml_rust::{Yaml, YamlLoader};

use crate::Context;

fn combine_yaml_scmd_arrays(overrider: Yaml, overriden: Yaml) -> Yaml {
    let mut result = Vec::new();
    let mut result_map = BTreeMap::new();
    let overrider_vec = overrider
        .into_vec()
        .expect("Subcommands should be an array");
    let overriden_vec = overriden
        .into_vec()
        .expect("Subcommands should be an array");
    for value in overrider_vec {
        let scmd_hash = value
            .clone()
            .into_hash()
            .expect(&format!("Invalid subcommand {:?}", value));
        let scmd_name = scmd_hash
            .keys()
            .nth(0)
            .expect(&format!("Invalid subcommand name {:?}", scmd_hash));
        result_map.insert(scmd_name.to_owned(), true);
        result.push(value);
    }
    for v in overriden_vec {
        let scmd_hash = v.as_hash().expect(&format!("Invalid subcommand {:?}", v));
        let scmd_name = scmd_hash
            .keys()
            .nth(0)
            .expect(&format!("Invalid subcommand name {:?}", scmd_hash));

        if !result_map.contains_key(scmd_name) {
            result.push(v);
        }
    }
    return Yaml::Array(result);
}

fn merge_btreemaps(
    overrider: &BTreeMap<Yaml, Yaml>,
    overriden: &BTreeMap<Yaml, Yaml>,
) -> BTreeMap<Yaml, Yaml> {
    let mut result = BTreeMap::new();
    for (k, v) in overrider.iter() {
        result.insert(k.clone(), v.clone());
    }
    for (k, v) in overriden.iter() {
        if !result.contains_key(k) {
            result.insert(k.clone(), v.clone());
        }
    }
    return result;
}

fn merge_scmd_hash(overrider: &BTreeMap<Yaml, Yaml>, overriden: &BTreeMap<Yaml, Yaml>) -> Yaml {
    let sub_key = Yaml::String(String::from("subcommands"));
    let scmds = combine_yaml_scmd_arrays(overrider[&sub_key].clone(), overriden[&sub_key].clone());

    let mut new_config = merge_btreemaps(overrider, overriden);
    new_config.insert(sub_key, scmds);
    return Yaml::Hash(new_config);
}

fn combine_scmd_yaml(overrider: &Yaml, overriden: &Yaml) -> Yaml {
    match (overrider, overriden) {
        (Yaml::Hash(r), Yaml::Hash(n)) => merge_scmd_hash(&r, &n),
        _ => return overrider.clone(),
    }
}

pub fn combine_hash_yaml(overrider: &Yaml, overriden: &Yaml) -> Yaml {
    let combined = match (overrider, overriden) {
        (Yaml::Hash(r), Yaml::Hash(n)) => merge_btreemaps(&r, &n),
        (Yaml::Hash(r), Yaml::BadValue) => merge_btreemaps(&r, &BTreeMap::new()),
        (Yaml::BadValue, Yaml::Hash(n)) => merge_btreemaps(&n, &BTreeMap::new()),
        _ => return overrider.clone(),
    };
    return Yaml::Hash(combined);
}

fn add_subcommands_path(config: Yaml, path: &String) -> Yaml {
    let mut config_bmap = get_imut_yaml_hash(config);
    let scmd_yaml = config_bmap
        .get_mut(&get_yaml_string("subcommands"))
        .expect("No subcommands in config, wrong yml format");
    let scmds = get_yaml_array(scmd_yaml);

    for scmd_yaml in scmds.iter_mut() {
        let scmd = get_yaml_hash(scmd_yaml);

        for (_scmd_name, scmd_options_yaml) in scmd.iter_mut() {
            let scmd_options = get_yaml_hash(scmd_options_yaml);
            let scmd_config_base_path = get_yaml_string("scmd_config_base_path");
            let path_value = get_yaml_string(&path);
            scmd_options.insert(scmd_config_base_path, path_value);
        }
    }

    Yaml::Hash(config_bmap.clone())
}

fn get_yaml_from(config_file_path: String, base_path: String) -> Yaml {
    let local_config =
        fs::read_to_string(config_file_path.clone()).expect("Could not find configuration file");
    let local_yaml =
        &YamlLoader::load_from_str(&local_config).expect("failed to load YAML file")[0];
    add_subcommands_path(local_yaml.clone(), &base_path)
}

fn get_config_from(app_name: &String, base_dir: &String) -> Option<Yaml> {
    let config_path = String::from(format!("{}/.{}.joat/", base_dir, app_name));
    let local_path = String::from(format!("{}{}.yml", config_path, app_name));
    if Path::new(&local_path).exists() {
        let config = get_yaml_from(local_path, config_path);
        return Some(config);
    }
    // To ease development
    let alternative_path = String::from(format!("{}/{}.yml", base_dir, app_name));
    if Path::new(&alternative_path).exists() {
        let config = get_yaml_from(alternative_path, base_dir.to_string());
        return Some(config);
    }
    return None;
}

fn get_config_vec_from_path(app_name: &String, path: &PathBuf) -> Vec<Yaml> {
    let mut ancestors = path.ancestors();
    let mut config_files = Vec::new();
    while let Some(path) = ancestors.next() {
        let current_dir = path.to_str().expect("Could not convert path to string");
        debug!("Searching config file in {:?}", current_dir);
        match get_config_from(app_name, &current_dir.to_string()) {
            Some(c) => config_files.push(c),
            None => continue,
        }
    }
    config_files
}

fn get_config_vec(app_name: &String) -> Vec<Yaml> {
    let current_path = env::current_dir().expect("Could not find current dir");
    let config_files = get_config_vec_from_path(app_name, &current_path);
    if config_files.is_empty() {
        let home_dir = dirs::home_dir().expect("No home folder");
        return get_config_vec_from_path(app_name, &home_dir);
    }
    config_files
}

fn get_yaml_string(rust_str: &str) -> Yaml {
    Yaml::String(String::from(rust_str))
}

fn get_arg_option(short: &str, long: &str, help: &str, takes_value: bool) -> BTreeMap<Yaml, Yaml> {
    let mut template_option = BTreeMap::new();

    let short_yaml = get_yaml_string("short");
    let short_value = get_yaml_string(short);
    let long_yaml = get_yaml_string("long");
    let long_value = get_yaml_string(long);
    let help_yaml = get_yaml_string("help");
    let help_value = get_yaml_string(help);
    let takes_value_yaml = get_yaml_string("takes_value");
    let takes_value_value = Yaml::Boolean(takes_value);

    template_option.insert(short_yaml, short_value);
    template_option.insert(long_yaml, long_value);
    template_option.insert(help_yaml, help_value);
    template_option.insert(takes_value_yaml, takes_value_value);

    template_option
}

fn get_template_arg_option() -> BTreeMap<Yaml, Yaml> {
    get_arg_option("t", "template", "Change the output template", true)
}

fn get_quiet_arg_option() -> BTreeMap<Yaml, Yaml> {
    get_arg_option("q", "quiet", "Do not output", false)
}

fn get_raw_arg_option() -> BTreeMap<Yaml, Yaml> {
    get_arg_option(
        "R",
        "raw_response",
        "Do not parse response with template",
        false,
    )
}

fn get_arg_yaml(name: &str, options: BTreeMap<Yaml, Yaml>) -> Yaml {
    let name = get_yaml_string(name);
    let mut args = BTreeMap::new();
    args.insert(name, Yaml::Hash(options));
    Yaml::Hash(args)
}

fn get_yaml_hash(yaml: &mut Yaml) -> &mut BTreeMap<Yaml, Yaml> {
    match *yaml {
        Yaml::Hash(ref mut h) => h,
        _ => {
            panic!("Failed to convert {:?} to mutable Yaml::Hash", yaml);
        }
    }
}

fn get_imut_yaml_hash(yaml: Yaml) -> BTreeMap<Yaml, Yaml> {
    match yaml {
        Yaml::Hash(h) => h,
        _ => {
            panic!("Failed to convert {:?} to immutable Yaml::Hash", yaml);
        }
    }
}

fn get_yaml_array(yaml: &mut Yaml) -> &mut Vec<Yaml> {
    match yaml {
        Yaml::Array(a) => a,
        _ => {
            panic!("Failed to convert {:?} to mutable Yaml::Array", yaml);
        }
    }
}

fn check_existing_options(args: Vec<Yaml>, option: &BTreeMap<Yaml, Yaml>) {
    for arg in args {
        let arg_hash = get_imut_yaml_hash(arg);
        for (_arg_name, arg_options_yaml) in arg_hash {
            let arg_options = get_imut_yaml_hash(arg_options_yaml);

            let short = get_yaml_string("short");
            match option.get(&short) {
                Some(o) => {
                    match arg_options.get(&short) {
                        Some(short_value) => {
                            if short_value == o {
                                println!("'short: {:?}' is reserved, check your yml", o);
                                ::std::process::exit(1);
                            }
                        }
                        None => (),
                    };
                }
                None => (),
            };

            let long = get_yaml_string("long");
            match option.get(&long) {
                Some(o) => {
                    match arg_options.get(&long) {
                        Some(long_value) => {
                            if long_value == o {
                                println!("'long: {:?}' is reserved, check your yml", o);
                                ::std::process::exit(1);
                            }
                        }
                        None => (),
                    };
                }
                None => (),
            };
        }
    }
}

fn add_auto_complete_cmd() -> Yaml {
    let mut auto_complete_cmd = BTreeMap::new();
    let mut auto_complete_cmd_options = BTreeMap::new();
    let cmd_string = Yaml::String("auto_complete".to_string());
    let about = Yaml::String("about".to_string());
    let about_description = Yaml::String("Create auto complete script".to_string());

    let mut args = Vec::new();
    let mut shell_arg = BTreeMap::new();
    let mut shell_arg_options = BTreeMap::new();

    let args_str = Yaml::String("args".to_string());
    let shell_arg_str = Yaml::String("SHELL".to_string());
    let shell_arg_help_key_str = Yaml::String("help".to_string());
    let shell_arg_help_value_str = Yaml::String("Which shell".to_string());
    let shell_arg_required_key_str = Yaml::String("required".to_string());
    let shell_arg_required_value_str = Yaml::Boolean(true);

    shell_arg_options.insert(shell_arg_help_key_str, shell_arg_help_value_str);
    shell_arg_options.insert(shell_arg_required_key_str, shell_arg_required_value_str);

    shell_arg.insert(shell_arg_str, Yaml::Hash(shell_arg_options));
    args.push(Yaml::Hash(shell_arg));

    auto_complete_cmd_options.insert(about, about_description);
    auto_complete_cmd_options.insert(args_str, Yaml::Array(args));
    auto_complete_cmd.insert(cmd_string, Yaml::Hash(auto_complete_cmd_options));
    Yaml::Hash(auto_complete_cmd)
}

fn add_default_options(config: Yaml) -> Yaml {
    let mut config_bmap = get_imut_yaml_hash(config);
    let scmd_yaml = config_bmap
        .get_mut(&get_yaml_string("subcommands"))
        .expect("No subcommands in config, wrong yml format");
    let scmds = get_yaml_array(scmd_yaml);

    for scmd_yaml in scmds.iter_mut() {
        let scmd = get_yaml_hash(scmd_yaml);

        for (_scmd_name, scmd_options_yaml) in scmd.iter_mut() {
            let scmd_options = get_yaml_hash(scmd_options_yaml);
            let args_yaml = get_yaml_string("args");
            if !scmd_options.contains_key(&args_yaml) {
                // If no arguments present add args key
                let args_vec: Vec<Yaml> = Vec::new();
                let args = Yaml::Array(args_vec);
                let iargs_yaml = get_yaml_string("args");
                scmd_options.insert(iargs_yaml, args);
            }
            let scmd_options_clone = scmd_options.clone();
            let args_opt = scmd_options.get_mut(&args_yaml).unwrap();
            let args = get_yaml_array(args_opt);

            let script_yaml = get_yaml_string("script");
            if !scmd_options_clone.contains_key(&script_yaml) {
                // Only non script subcommands get the template option
                let template_option = get_template_arg_option();
                check_existing_options(args.clone(), &template_option);
                args.push(get_arg_yaml("template", template_option));
            }

            let quiet_option = get_quiet_arg_option();
            check_existing_options(args.clone(), &quiet_option);
            args.push(get_arg_yaml("quiet", quiet_option));

            let raw_option = get_raw_arg_option();
            check_existing_options(args.clone(), &raw_option);
            args.push(get_arg_yaml("raw_response", raw_option));
        }
    }
    let auto_complete_cmd = add_auto_complete_cmd();
    scmds.push(auto_complete_cmd);

    Yaml::Hash(config_bmap.clone())
}

fn override_version(app_name: &String, config: Yaml) -> Yaml {
    let version;
    if app_name == env!("CARGO_PKG_NAME") {
        version = String::from(config["version"].as_str().expect("Version not defined"));
    } else {
        let joat_version = env!("CARGO_PKG_VERSION");
        let app_version = config["version"].as_str().expect("Version not defined");
        version = format!("{} (joat {})", app_version, joat_version);
    }
    let mut config_btree = config.into_hash().expect("Config yaml is not a hash");
    let version_yaml = get_yaml_string("version");
    let version_value_yaml = get_yaml_string(&version);
    config_btree.insert(version_yaml, version_value_yaml);
    Yaml::Hash(config_btree)
}

fn create_default_config() {
    let yaml_string = String::from(include_str!("../joat.yml"));
    let home_dir_path = match dirs::home_dir() {
        Some(h) => h,
        _ => panic!("No home dir"),
    };
    let home_dir_str = home_dir_path.into_os_string().into_string().unwrap();
    let joat_config_path = format!("{}/.joat.joat", home_dir_str);
    match fs::create_dir_all(&joat_config_path) {
        Ok(_v) => (),
        Err(e) => panic!("Could not create config folder {:?}", e),
    };

    let filename = format!("{}/joat.yml", joat_config_path);
    fs::write(filename, &yaml_string).expect("Unable to write file");
}

pub fn get_yaml_config(app_name: &String) -> Yaml {
    let config_vec = get_config_vec(app_name);
    let mut combined_config: Option<Yaml> = None;
    debug!("Config vec {:?}", config_vec);
    for current_config in config_vec {
        match combined_config {
            Some(c) => {
                combined_config = Some(combine_scmd_yaml(&c, &current_config));
            }
            None => {
                combined_config = Some(current_config);
            }
        }
    }

    let partial_config = match combined_config {
        Some(c) => c,
        None => {
            if app_name == "joat" {
                create_default_config();
                return get_yaml_config(&app_name);
            }
            panic!("Could not find config file");
        }
    };
    let config = override_version(app_name, partial_config);
    add_default_options(config)
}

pub fn get_string_from_yaml(yaml: &Yaml) -> String {
    match yaml.clone().into_string() {
        Some(s) => s,
        None => {
            panic!("Failed to convert {:?} into string, exiting.", yaml);
        }
    }
}

fn get_value_from_yaml_hash(btree_map: &BTreeMap<Yaml, Yaml>, context: &Context) -> Value {
    let mut value_map = Map::new();
    for (key, value) in btree_map.iter() {
        let key_str = get_string_from_yaml(key);
        let v_value = match get_value_from_yaml(value, context) {
            Some(v) => v,
            None => continue,
        };
        value_map.insert(key_str, v_value);
    }
    Value::from(value_map)
}

fn get_value_from_yaml_array(yaml_vec: &Vec<Yaml>, context: &Context) -> Value {
    let mut value_vec = Vec::new();
    for value in yaml_vec.iter() {
        let v_value = match get_value_from_yaml(value, context) {
            Some(v) => v,
            None => continue,
        };
        value_vec.push(v_value);
    }
    Value::from(value_vec)
}

fn get_value_from_yaml_string(yaml_str: &String, context: &Context) -> Option<Value> {
    let templated_str = match template::get_compiled_template_str_with_context(&yaml_str, context) {
        Ok(t) => t,
        Err(_e) => return None,
    };
    if templated_str == "true" || templated_str == "false" {
        let bool_value =
            FromStr::from_str(&templated_str).expect("Could not convert boolean value in body");
        return Some(Value::Bool(bool_value));
    }
    let value = match serde_json::from_str::<Number>(&templated_str) {
        Ok(n) => Value::Number(n),
        Err(_e) => Value::String(templated_str),
    };
    Some(value)
}

pub fn get_value_from_yaml(yaml: &Yaml, context: &Context) -> Option<Value> {
    match yaml {
        Yaml::Hash(y) => Some(get_value_from_yaml_hash(y, context)),
        Yaml::Array(y) => Some(get_value_from_yaml_array(y, context)),
        Yaml::String(y) => get_value_from_yaml_string(y, context),
        Yaml::Integer(y) => Some(Value::from(y.clone())),
        Yaml::Boolean(y) => Some(Value::from(y.clone())),
        _ => Some(Value::Null),
    }
}

pub fn get_hash_from_yaml(yaml: &Yaml, context: &Context, deep: bool) -> HashMap<String, Value> {
    if yaml.is_badvalue() {
        return HashMap::new();
    }
    let yaml_btree = yaml
        .clone()
        .into_hash()
        .expect("Yaml should be a map value");
    let mut yaml_hash = HashMap::new();
    for (key, value) in yaml_btree.iter() {
        let str_key = get_string_from_yaml(key);
        if deep {
            let v_value = match get_value_from_yaml(value, context) {
                Some(v) => v,
                None => continue,
            };
            yaml_hash.insert(str_key, v_value);
        } else {
            let raw_string = match value {
                Yaml::String(v) => v,
                _ => panic!("Only string values are allowed for {}", str_key),
            };
            let str_option = template::get_compiled_template_str_with_context(&raw_string, context);
            let str_value = match str_option {
                Ok(s) => s,
                Err(_e) => continue,
            };
            yaml_hash.insert(str_key, Value::String(str_value));
        }
    }
    return yaml_hash;
}

pub fn get_subcommand_from_yaml(cmd_name: &str, yaml: &Yaml) -> Yaml {
    let subcommands = &yaml["subcommands"];
    let subcommands_vec = match subcommands.clone().into_vec() {
        Some(t) => t,
        None => {
            panic!("Failed to retrieve subcommands, exiting.");
        }
    };
    let cmd_name_yaml = Yaml::from_str(cmd_name);
    let scmd_option = subcommands_vec
        .iter()
        .find(|&s| match s.clone().into_hash() {
            Some(sl) => sl.contains_key(&cmd_name_yaml),
            None => false,
        });
    let scmd_hash = match scmd_option {
        Some(s) => s,
        None => {
            panic!("Failed to retrieve subcommands hash, exiting.");
        }
    };
    return scmd_hash[cmd_name].clone();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample_subcommand(name: &str) -> Yaml {
        let mut scmd_btree = BTreeMap::new();
        let mut scmd_options_btree = BTreeMap::new();

        let name = get_yaml_string(name);
        let about = get_yaml_string("about");
        let about_value = get_yaml_string("This is a sample scmd");

        scmd_options_btree.insert(about, about_value);
        scmd_btree.insert(name, Yaml::Hash(scmd_options_btree));

        Yaml::Hash(scmd_btree)
    }

    fn create_sample_yaml(name_value: &str) -> Yaml {
        let mut yaml_btree = BTreeMap::new();

        let name = get_yaml_string("name");
        let name_value = get_yaml_string(name_value);
        let subcommands_label = get_yaml_string("subcommands");
        let mut subcommands = Vec::new();
        subcommands.push(create_sample_subcommand("scmd1"));
        subcommands.push(create_sample_subcommand("scmd2"));

        yaml_btree.insert(name, name_value);
        yaml_btree.insert(subcommands_label, Yaml::Array(subcommands));
        Yaml::Hash(yaml_btree)
    }

    #[test]
    fn test_get_yaml_string() {
        // Arrange
        let sample_str = "sample_str";
        let yaml_str = Yaml::String(sample_str.to_string());

        // Act, Assert
        assert_eq!(yaml_str, get_yaml_string(sample_str));
    }

    #[test]
    fn test_get_string_from_yaml() {
        // Arrange
        let sample_str = "sample_str";
        let yaml_str = Yaml::String(sample_str.to_string());

        // Act, Assert
        assert_eq!(sample_str, get_string_from_yaml(&yaml_str));
    }

    #[test]
    #[should_panic]
    fn test_get_string_from_yaml_non_str_yaml() {
        // Arrange
        let yaml_number = Yaml::Integer(1);

        // Act, Assert
        get_string_from_yaml(&yaml_number);
    }

    #[test]
    fn test_get_subcommand_from_yaml() {
        // Arrange
        let yaml = create_sample_yaml("some random value");
        let scmd_about = "This is a sample scmd";

        // Act
        let subcommand = get_subcommand_from_yaml("scmd2", &yaml);

        // Assert
        let scmd_btree = subcommand.into_hash().expect("Could not cast to btree");
        let about = &scmd_btree[&get_yaml_string("about")];
        assert_eq!(about, &get_yaml_string(scmd_about));
    }

    #[test]
    fn test_combine_scmd_yaml_override_props() {
        // Arrange
        let final_value = "some random value";
        let other_value = "some other value";
        let overrider = create_sample_yaml(final_value);
        let overriden = create_sample_yaml(other_value);

        // Act
        let result = combine_scmd_yaml(&overrider, &overriden);

        // Assert
        assert_eq!(result["name"], get_yaml_string(final_value));
        assert_ne!(result["name"], get_yaml_string(other_value));
    }
}
