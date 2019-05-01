use reqwest::Response;
use std::collections::HashMap;
use yaml_rust::Yaml;
use std::vec::Vec;
use clap::ArgMatches;

use crate::yaml;
use crate::template;

fn get_complete_endpoint(base_endpoint: &Yaml, path_yaml: &Yaml) -> String {
    let mut endpoint = yaml::get_string_from_yaml(base_endpoint);
    let path_str = yaml::get_string_from_yaml(path_yaml);
    endpoint.push_str(&path_str);
    return endpoint;
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

fn get_param_split(param: String) -> (String, String) {
    let mut parts = param.split("=");
    if parts.clone().count() != 2 {
        println!("Params should be formatted as foo=bar");
        ::std::process::exit(1);
    }
    let raw_key = &String::from(parts.next().unwrap());
    let raw_value = &String::from(parts.next().unwrap());
    let key = template::get_compiled_template_str(raw_key);
    let value = template::get_compiled_template_str(raw_value);
    return (key, value);
}

fn stringify(query: Vec<(String, String)>) -> String {
    query.iter().fold(String::new(), |acc, tuple| {
        acc + &tuple.0 + "=" + &tuple.1 + "&"
    })
}

fn get_params(args: &ArgMatches) -> String {
    let mut param_vec = Vec::new();
    let params = match args.values_of("param") {
        Some(t) => t,
        None => {
            return String::from("");
        },
    };
    for p in params {
        let param = String::from(p);
        let splitted_param = get_param_split(param);
        param_vec.push(splitted_param);
    }
    return stringify(param_vec);
}

pub fn get_endpoint(cmd_name: &str, args: &ArgMatches, yaml: &Yaml) -> String {
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);
    let raw_endpoint = get_complete_endpoint(&yaml["base_endpoint"], &subcmd_yaml["path"]);
    let params = get_params(args);
    let args = get_args_context(&args, &subcmd_yaml);
    let mut parsed_endpoint = template::get_compiled_template_str_with_context(&raw_endpoint, args);
    if params.len() > 0 {
        parsed_endpoint = format!("{}?{}", parsed_endpoint, params);
    }
    return parsed_endpoint;
}

pub fn get_resource(endpoint: &String, headers: &HashMap<String, String>) -> Response {
    let client = reqwest::Client::new();
    let mut client_get = client.get(endpoint);
    for (name, value) in headers.iter() {
        client_get = client_get.header(&name[..], &value[..]);
    }
    let response = match client_get.send() {
        Ok(t) => t,
        Err(e) => {
            println!("Could not get response for endpoint {}", endpoint);
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };
    return response;
}