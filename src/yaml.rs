use crate::template;
use serde_json::map::Map;
use serde_json::value::Value;
use serde_json::Number;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use yaml_rust::{Yaml, YamlLoader};

use crate::Context;

fn merge_hash(overrider: &BTreeMap<Yaml, Yaml>, overriden: &BTreeMap<Yaml, Yaml>) -> Yaml {
    let sub_key = Yaml::String(String::from("subcommands"));
    let r_subcmd = overrider[&sub_key]
        .clone()
        .into_vec()
        .expect("Subcommands should be an array");
    let mut cmds = Vec::new();
    let mut r_map = BTreeMap::new();
    for value in r_subcmd {
        let scmd_hash = value
            .clone()
            .into_hash()
            .expect(&format!("Invalid subcommand {:?}", value));
        let scmd_name = scmd_hash
            .keys()
            .nth(0)
            .expect(&format!("Invalid subcommand name {:?}", scmd_hash));
        r_map.insert(scmd_name.to_owned(), true);
        cmds.push(value);
    }
    let n_subcmd = overriden[&sub_key]
        .clone()
        .into_vec()
        .expect("Subcommands should be an array");
    for v in n_subcmd {
        let scmd_hash = v.as_hash().expect(&format!("Invalid subcommand {:?}", v));
        let scmd_name = scmd_hash
            .keys()
            .nth(0)
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

fn get_config_from(cmd_name: &String, base_dir: &String) -> Option<Yaml> {
    let local_path = String::from(format!("{}{}.yml", base_dir, cmd_name));
    if Path::new(&local_path).exists() {
        let local_config =
            fs::read_to_string(local_path.clone()).expect("Could not find configuration file");
        let local_yaml =
            &YamlLoader::load_from_str(&local_config).expect("failed to load YAML file")[0];
        let config = add_subcommands_path(local_yaml.clone(), base_dir);
        return Some(config);
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

fn get_yaml_string(rust_str: &str) -> Yaml {
    Yaml::String(String::from(rust_str))
}

fn get_template_arg_options() -> BTreeMap<Yaml, Yaml> {
    let mut template_options = BTreeMap::new();

    let short = get_yaml_string("short");
    let short_value = get_yaml_string("t");
    let long = get_yaml_string("long");
    let long_value = get_yaml_string("template");
    let help = get_yaml_string("help");
    let help_value = get_yaml_string("Change the output template");
    let takes_value = get_yaml_string("takes_value");
    let takes_value_value = Yaml::Boolean(true);

    template_options.insert(short, short_value);
    template_options.insert(long, long_value);
    template_options.insert(help, help_value);
    template_options.insert(takes_value, takes_value_value);

    template_options
}

fn get_quiet_arg_options() -> BTreeMap<Yaml, Yaml> {
    let mut quiet_options = BTreeMap::new();

    let short = get_yaml_string("short");
    let short_value = get_yaml_string("q");
    let long = get_yaml_string("long");
    let long_value = get_yaml_string("quiet");
    let help = get_yaml_string("help");
    let help_value = get_yaml_string("Do not output");

    quiet_options.insert(short, short_value);
    quiet_options.insert(long, long_value);
    quiet_options.insert(help, help_value);

    quiet_options
}

fn get_raw_arg_options() -> BTreeMap<Yaml, Yaml> {
    let mut raw_options = BTreeMap::new();

    let short = get_yaml_string("short");
    let short_value = get_yaml_string("R");
    let long = get_yaml_string("long");
    let long_value = get_yaml_string("raw_response");
    let help = get_yaml_string("help");
    let help_value = get_yaml_string("Do not parse response as template");

    raw_options.insert(short, short_value);
    raw_options.insert(long, long_value);
    raw_options.insert(help, help_value);

    raw_options
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
                let template_options = get_template_arg_options();
                check_existing_options(args.clone(), &template_options);
                args.push(get_arg_yaml("template", template_options));
            }

            let quiet_options = get_quiet_arg_options();
            check_existing_options(args.clone(), &quiet_options);
            args.push(get_arg_yaml("quiet", quiet_options));

            let raw_options = get_raw_arg_options();
            check_existing_options(args.clone(), &raw_options);
            args.push(get_arg_yaml("raw_response", raw_options));
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
        let joat_version = env!("CARGO_PKG_VERSION");;
        let app_version = config["version"].as_str().expect("Version not defined");
        version = format!("{} (joat {})", app_version, joat_version);
    }
    let mut config_btree = config.into_hash().expect("Config yaml is not a hash");
    let version_yaml = get_yaml_string("version");
    let version_value_yaml = get_yaml_string(&version);
    config_btree.insert(version_yaml, version_value_yaml);
    Yaml::Hash(config_btree)
}

pub fn get_yaml_config(app_name: &String) -> Yaml {
    let home_config = get_home_config(app_name);
    let local_config = get_local_config(app_name);
    let partial_config = match (home_config, local_config) {
        (Some(h), Some(l)) => combine_yaml(&h, &l),
        (Some(h), None) => h,
        (None, Some(l)) => l,
        (None, None) => {
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

fn get_value_from_yaml(yaml: &Yaml, context: &Context) -> Option<Value> {
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

    fn create_sample_yaml() -> Yaml {
        let mut yaml_btree = BTreeMap::new();

        let name = get_yaml_string("name");
        let name_value = get_yaml_string("test");
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
        let yaml = create_sample_yaml();
        let scmd_about = "This is a sample scmd";

        // Act
        let subcommand = get_subcommand_from_yaml("scmd2", &yaml);

        // Assert
        let scmd_btree = subcommand.into_hash().expect("Could not cast to btree");
        let about = &scmd_btree[&get_yaml_string("about")];
        assert_eq!(about, &get_yaml_string(scmd_about));
    }
}
