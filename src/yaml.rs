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
        },
    }
}

fn get_imut_yaml_hash(yaml: Yaml) -> BTreeMap<Yaml, Yaml> {
    match yaml {
        Yaml::Hash(h) => h,
        _ => {
            panic!("Failed to convert {:?} to immutable Yaml::Hash", yaml);
        },
    }
}

fn get_yaml_array(yaml: &mut Yaml) -> &mut Vec<Yaml> {
    match yaml {
        Yaml::Array(a) => a,
        _ => {
            panic!("Failed to convert {:?} to mutable Yaml::Array", yaml);
        },
    }
}

fn check_existing_options(args: Vec<Yaml>, option: &BTreeMap<Yaml,Yaml>) {
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
                        },
                        None => ()
                    };
                },
                None => ()
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
                        },
                        None => ()
                    };
                },
                None => ()
            };
        }
    }
}

fn add_default_options(config: Yaml) -> Yaml {
    let mut config_bmap = get_imut_yaml_hash(config);
    let scmd_yaml = config_bmap.get_mut(&get_yaml_string("subcommands"))
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
    Yaml::Hash(config_bmap.clone())
}

pub fn get_yaml_config(cmd_name: &String) -> Yaml {
    let home_config = get_home_config(cmd_name);
    let local_config = get_local_config(cmd_name);
    let config = match (home_config, local_config) {
        (Some(h), Some(l)) => combine_yaml(&h, &l),
        (Some(h), None) => h,
        (None, Some(l)) => l,
        (None, None) => {
            println!("Could not find config file");
            ::std::process::exit(1);
        }
    };
    add_default_options(config)
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