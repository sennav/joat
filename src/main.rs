#[macro_use]
extern crate clap;
extern crate reqwest;
extern crate serde_json;
extern crate tera;
extern crate yaml_rust;
extern crate serde;

use clap::App;
use clap::ArgMatches;
use serde_json::value::Value;
use serde_json::map::Map;
use std::collections::HashMap;
use std::env;
use yaml_rust::Yaml;

mod template;
mod http;
mod yaml;

fn get_env_hash() -> Value {
    let mut env_vars = Map::new();
    for (key, value) in env::vars() {
        let v: Value = value.into();
        env_vars.insert(key, v);
    }
    return Value::from(env_vars);
}

fn get_args_context(args: &ArgMatches, subcmd_yaml: &Yaml) -> HashMap<String, String> {
    let mut args_context = HashMap::new();
    for arg in subcmd_yaml["args"].clone().into_iter() {
        for a in arg.into_hash().unwrap().keys() {
            let key = a.clone().into_string().unwrap();
            if args.is_present(&key) {
                let value = args.value_of(&key).unwrap_or("");
                args_context.insert(key, String::from(value));
            }
        }
    }
    return args_context;
}

fn execute(cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);
    let raw_endpoint = http::get_complete_endpoint(&yaml["base_endpoint"], &subcmd_yaml["path"]);
    let params = http::get_params(args);
    let args = get_args_context(&args, &subcmd_yaml);
    let mut parsed_endpoint = template::get_compiled_template_str_with_context(&raw_endpoint, args);
    if params.len() > 0 {
        parsed_endpoint = format!("{}?{}", parsed_endpoint, params);
    }
    let headers = yaml::get_hash_from_yaml(&yaml["headers"]);
    let mut response = http::get_resource(&parsed_endpoint, &headers);
    let result: Value = match response.json() {
        Ok(r) => r,
        Err(e) => {
            println!("Could not convert response {:?} to json", response);
            println!("Error {:?}", e);
            ::std::process::exit(1);
        },
    };
    let mut response_context = HashMap::new();
    response_context.insert(String::from("response"), result);
    response_context.insert(String::from("env"), get_env_hash());
    let subcmd_hash = subcmd_yaml.clone().into_hash().unwrap();
    let mut template: String;
    if subcmd_hash.contains_key(&Yaml::from_str("template")) {
        template = subcmd_yaml["template"].clone().into_string().unwrap();
    } else {
        template = String::from("debug")
    }
    print!("{}", template::get_compiled_template_with_context(template, response_context));
}

fn main() {
    let yaml = load_yaml!("../cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    match matches.subcommand() {
        (name, sub_cmd_option) => {
            match sub_cmd_option {
                Some(sub_cmd) => execute(name, sub_cmd, yaml),
                _ => ::std::process::exit(1)
            }
        }
    }
}